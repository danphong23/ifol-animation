use super::validation::validate_copy_range;
use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    ComputeCommand, CopyCommand, GraphResource, RenderGraph, RenderNodePool, RenderTarget,
    ResourceAccess, ResourceSubresource,
};
use crate::resources::{
    BufferHandle, BufferResourceDescriptor, ComputePipelineHandle, RenderNodeId, ResourceRegistry,
    TextureHandle,
};

#[test]
fn validation_rejects_compute_node_without_pipeline() {
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_compute_batch(
        &mut pool,
        vec![ComputeCommand::new(ComputePipelineHandle(42), [1, 1, 1])],
    );

    assert_eq!(
        RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
        Err(RenderGraphValidationError::MissingComputePipeline(
            ComputePipelineHandle(42),
        ))
    );
}

#[test]
fn validation_rejects_copy_node_without_buffer() {
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(1),
            BufferHandle(2),
            16,
        )],
    );

    assert_eq!(
        RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
        Err(RenderGraphValidationError::MissingBuffer(BufferHandle(1)))
    );
}

#[test]
fn validation_rejects_declared_usage_without_resource() {
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let node = graph.add_copy_batch(&mut pool, vec![]);
    graph.declare_resource_usage(
        node,
        GraphResource::Buffer(BufferHandle(404)),
        ResourceAccess::Read,
    );
    assert_eq!(
        RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
        Err(RenderGraphValidationError::MissingUsageBuffer(
            BufferHandle(404)
        ))
    );
}

#[test]
fn declared_resource_usage_is_preserved_on_graph() {
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let node = RenderNodeId(9);
    graph.declare_resource_usage(
        node,
        GraphResource::Texture(TextureHandle(7)),
        ResourceAccess::ReadWrite,
    );
    assert_eq!(
        graph.resource_usages(&node),
        &[crate::graph::ResourceUsage {
            resource: GraphResource::Texture(TextureHandle(7)),
            access: ResourceAccess::ReadWrite,
            subresource: ResourceSubresource::Whole,
        }]
    );
}

#[test]
fn validation_rejects_buffer_copy_with_missing_usage_bits() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let source = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("invalid_copy_source"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let destination = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("invalid_copy_destination"),
        size: 16,
        usage: wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut registry = ResourceRegistry::new();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(1),
            source,
            BufferResourceDescriptor {
                size: 16,
                usage: wgpu::BufferUsages::COPY_DST,
            },
        )
        .unwrap();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(2),
            destination,
            BufferResourceDescriptor {
                size: 16,
                usage: wgpu::BufferUsages::COPY_SRC,
            },
        )
        .unwrap();
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(1),
            BufferHandle(2),
            4,
        )],
    );
    assert_eq!(
        RenderGraphExecutor::new().validate(&registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingBufferUsage {
            handle: BufferHandle(1),
            required_usage: wgpu::BufferUsages::COPY_SRC.bits(),
            actual_usage: wgpu::BufferUsages::COPY_DST.bits(),
        })
    );
}

#[test]
fn copy_range_validation_checks_overflow_and_bounds() {
    assert!(validate_copy_range(BufferHandle(1), 8, 8, 16).is_ok());
    assert!(matches!(
        validate_copy_range(BufferHandle(1), 12, 8, 16),
        Err(RenderGraphValidationError::InvalidCopyRange { .. })
    ));
    assert!(matches!(
        validate_copy_range(BufferHandle(1), u64::MAX, 1, u64::MAX),
        Err(RenderGraphValidationError::InvalidCopyRange { .. })
    ));
}
