use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use crate::resources::handle::*;
use crate::resources::{
    BufferHandle, BufferResourceDescriptor, PipelineHandle, PipelineLayoutResourceDescriptor,
    ResourceRegistry,
};

fn render_pipeline(device: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
        )),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn registry_with_pipeline(engine: &crate::backend::GpuEngine) -> ResourceRegistry {
    let mut registry = ResourceRegistry::new();
    registry.insert_pipeline_with_layout_descriptor(
        PipelineHandle(1),
        render_pipeline(engine.device()),
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![],
        },
    );
    registry
}

#[test]
fn validation_rejects_missing_mesh_for_indexed_indirect() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = registry_with_pipeline(&engine);
    let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 16,
        usage: wgpu::BufferUsages::INDIRECT,
        mapped_at_creation: false,
    });
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(1),
            buffer,
            BufferResourceDescriptor {
                size: 16,
                usage: wgpu::BufferUsages::INDIRECT,
            },
        )
        .unwrap();

    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let id = pool.alloc_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::IndexedIndirect {
            mesh: MeshHandle(99),
            buffer: BufferHandle(1),
            offset: 0,
        },
    )]);
    graph.add_node_id(id);

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingMesh(MeshHandle(99)))
    );
}

#[test]
fn validation_rejects_missing_buffer_for_indirect() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = registry_with_pipeline(&engine);
    let index_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 2,
        usage: wgpu::BufferUsages::INDEX,
        mapped_at_creation: false,
    });
    let vertex_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::VERTEX,
        mapped_at_creation: false,
    });
    registry
        .insert_mesh_with_descriptor(
            MeshHandle(1),
            (
                vertex_buffer,
                Some((index_buffer, wgpu::IndexFormat::Uint16)),
                1,
            ),
            crate::resources::MeshResourceDescriptor {
                vertex_count: 1,
                index_buffer_size: Some(2),
                index_format: Some(wgpu::IndexFormat::Uint16),
                vertex_buffer_size: 4,
            },
        )
        .unwrap();

    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let id = pool.alloc_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::IndexedIndirect {
            mesh: MeshHandle(1),
            buffer: BufferHandle(99),
            offset: 0,
        },
    )]);
    graph.add_node_id(id);

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingIndirectBuffer(
            BufferHandle(99)
        ))
    );
}

#[test]
fn validation_rejects_missing_mesh_for_indexed() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let registry = registry_with_pipeline(&engine);

    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let id = pool.alloc_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Indexed {
            mesh: MeshHandle(99),
            index_range: 0..3,
            instance_range: 0..1,
        },
    )]);
    graph.add_node_id(id);

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingMesh(MeshHandle(99)))
    );
}
