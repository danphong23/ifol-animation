use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use crate::resources::{
    BindGroupHandle, BindGroupResourceDescriptor, ComputePipelineHandle, PipelineHandle,
    PipelineLayoutResourceDescriptor, ResourceRegistry,
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

fn empty_bind_group(device: &wgpu::Device) -> wgpu::BindGroup {
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[],
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[],
    })
}

#[test]
fn validation_rejects_render_pipeline_layout_mismatch() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = ResourceRegistry::new();
    registry.insert_pipeline_with_layout_descriptor(
        PipelineHandle(1),
        render_pipeline(engine.device()),
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![Some(10)],
        },
    );
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(1),
            empty_bind_group(engine.device()),
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 11,
            },
        )
        .unwrap();

    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let id = pool.alloc_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..1,
        },
    )
    .with_bind_group(0, BindGroupHandle(1), vec![])]);
    graph.add_node_id(id);

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::PipelineLayoutMismatch {
            pipeline: PipelineHandle(1),
            slot: 0,
            expected: Some(10),
            actual: Some(11),
        })
    );
}

#[test]
fn validation_rejects_compute_pipeline_layout_mismatch() {
    let mut builder = GpuEngineBuilder::new();
    builder = builder.with_required_limits(wgpu::Limits {
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        max_compute_workgroup_size_y: 256,
        max_compute_workgroup_size_z: 64,
        ..Default::default()
    });
    let engine = pollster::block_on(builder.build()).unwrap();
    let shader = engine
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@compute @workgroup_size(1) fn main() {}",
            )),
        });
    let pipeline_layout = engine
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });
    let pipeline = engine
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

    let mut registry = ResourceRegistry::new();
    registry.insert_compute_pipeline_with_layout_descriptor(
        ComputePipelineHandle(1),
        pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![Some(10)],
        },
    );
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(1),
            empty_bind_group(engine.device()),
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 11,
            },
        )
        .unwrap();

    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    let id = pool.alloc_compute_batch(vec![ComputeCommand::new(
        ComputePipelineHandle(1),
        [1, 1, 1],
    )
    .with_bind_group(0, BindGroupHandle(1), vec![])]);
    graph.add_node_id(id);

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
            pipeline: ComputePipelineHandle(1),
            slot: 0,
            expected: Some(10),
            actual: Some(11),
        })
    );
}
