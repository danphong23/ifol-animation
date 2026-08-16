use super::validation::validate_indirect_buffer;
use super::{
    bundle_cache_key, format_has_stencil, texture_supports_aspect, RenderGraphExecutor,
    RenderGraphValidationError,
};
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    DrawAction, DrawCommand, RenderGraph, RenderNode, RenderNodePool, RenderTarget,
};
use crate::resources::{
    BufferHandle, BufferResourceDescriptor, PipelineHandle, ResourceRegistry, TextureHandle,
};

#[test]
fn stencil_aspect_detection_is_format_specific() {
    assert!(format_has_stencil(wgpu::TextureFormat::Stencil8));
    assert!(format_has_stencil(wgpu::TextureFormat::Depth24PlusStencil8));
    assert!(format_has_stencil(
        wgpu::TextureFormat::Depth32FloatStencil8
    ));
    assert!(!format_has_stencil(wgpu::TextureFormat::Depth24Plus));
    assert!(!format_has_stencil(wgpu::TextureFormat::Depth32Float));
}

#[test]
fn texture_copy_aspect_support_is_format_specific() {
    use crate::graph::TextureAspect;

    assert!(texture_supports_aspect(
        wgpu::TextureFormat::Depth24PlusStencil8,
        TextureAspect::DepthOnly
    ));
    assert!(texture_supports_aspect(
        wgpu::TextureFormat::Depth24PlusStencil8,
        TextureAspect::StencilOnly
    ));
    assert!(texture_supports_aspect(
        wgpu::TextureFormat::Stencil8,
        TextureAspect::StencilOnly
    ));
    assert!(!texture_supports_aspect(
        wgpu::TextureFormat::Rgba8Unorm,
        TextureAspect::DepthOnly
    ));
    assert!(!texture_supports_aspect(
        wgpu::TextureFormat::Depth32Float,
        TextureAspect::StencilOnly
    ));
}

#[test]
fn indirect_buffer_validation_checks_alignment_range_and_usage() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = ResourceRegistry::new();
    let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("indirect_validation"),
        size: 64,
        usage: wgpu::BufferUsages::INDIRECT,
        mapped_at_creation: false,
    });
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(70),
            buffer,
            BufferResourceDescriptor {
                size: 64,
                usage: wgpu::BufferUsages::INDIRECT,
            },
        )
        .unwrap();
    assert!(validate_indirect_buffer(&registry, BufferHandle(70), 0, 16).is_ok());
    assert!(matches!(
        validate_indirect_buffer(&registry, BufferHandle(70), 2, 16),
        Err(RenderGraphValidationError::InvalidIndirectRange { .. })
    ));
    assert!(matches!(
        validate_indirect_buffer(&registry, BufferHandle(70), 52, 16),
        Err(RenderGraphValidationError::InvalidIndirectRange { .. })
    ));
}

#[test]
fn validation_rejects_missing_offscreen_target() {
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(9),
        width: 64,
        height: 64,
    });
    let result = RenderGraphExecutor::new().validate(
        &ResourceRegistry::new(),
        &RenderNodePool::new(),
        &graph,
    );

    assert_eq!(
        result,
        Err(RenderGraphValidationError::MissingTexture(TextureHandle(9)))
    );
}

#[test]
fn public_execute_checked_rejects_invalid_graph_before_submit() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(9),
        width: 64,
        height: 64,
    });
    let result = RenderGraphExecutor::new().execute_checked(
        &engine,
        &ResourceRegistry::new(),
        &mut RenderNodePool::new(),
        &graph,
    );

    assert_eq!(
        result.err(),
        Some(RenderGraphValidationError::MissingTexture(TextureHandle(9)))
    );
}

#[test]
fn validate_with_device_exposes_adapter_aware_contract() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(9),
        width: 64,
        height: 64,
    });
    let result = RenderGraphExecutor::new().validate_with_device(
        &engine,
        &ResourceRegistry::new(),
        &RenderNodePool::new(),
        &graph,
    );
    assert_eq!(
        result,
        Err(RenderGraphValidationError::MissingTexture(TextureHandle(9)))
    );
}

#[test]
fn validation_rejects_zero_sized_target_before_resource_lookup() {
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(9),
        width: 0,
        height: 64,
    });
    let result = RenderGraphExecutor::new().validate(
        &ResourceRegistry::new(),
        &RenderNodePool::new(),
        &graph,
    );

    assert_eq!(
        result,
        Err(RenderGraphValidationError::InvalidTargetSize {
            width: 0,
            height: 64,
        })
    );
}

#[test]
fn bundle_key_changes_when_pipeline_version_changes() {
    let node = RenderNode::new_batch(vec![DrawCommand::new(
        PipelineHandle(7),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..1,
        },
    )]);
    let mut registry = ResourceRegistry::new();
    let first = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        1,
        0,
    );
    registry.mark_pipeline_changed(PipelineHandle(7));
    let second = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        1,
        0,
    );

    assert_ne!(first, second);
    let single_sample = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        1,
        0,
    );
    let msaa = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        4,
        0,
    );
    assert_ne!(single_sample, msaa);
    let context_a = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        1,
        11,
    );
    let context_b = bundle_cache_key(
        &node,
        &registry,
        wgpu::TextureFormat::Rgba8Unorm,
        None,
        1,
        22,
    );
    assert_ne!(context_a, context_b);
}
