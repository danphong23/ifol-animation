use super::RenderGraphExecutor;
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    ComputeCommand, CopyCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use crate::resources::{
    BufferHandle, BufferResourceDescriptor, ComputePipelineHandle, PipelineHandle,
    PipelineLayoutResourceDescriptor, ResourceRegistry,
};

#[test]
fn execution_empty_graph_does_not_crash() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let registry = ResourceRegistry::new();
    let mut pool = RenderNodePool::new();
    let graph = RenderGraph::new(RenderTarget::Screen);

    let executor = RenderGraphExecutor::new();
    assert_eq!(
        executor.validate_with_device(&engine, &registry, &pool, &graph),
        Ok(())
    );
    let report = executor
        .execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None)
        .unwrap();
    assert_eq!(report.flattened_nodes, 0);
    assert_eq!(report.draw_commands, 0);
}

#[test]
fn execution_deeply_nested_subgraphs() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let registry = ResourceRegistry::new();
    let mut pool = RenderNodePool::new();

    let child3 = RenderGraph::new(RenderTarget::Screen);
    let id3 = pool.alloc_subgraph("child3".to_string(), child3, vec![]);

    let mut child2 = RenderGraph::new(RenderTarget::Screen);
    child2.add_node_id(id3);
    let id2 = pool.alloc_subgraph("child2".to_string(), child2, vec![]);

    let mut child1 = RenderGraph::new(RenderTarget::Screen);
    child1.add_node_id(id2);
    let id1 = pool.alloc_subgraph("child1".to_string(), child1, vec![]);

    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_node_id(id1);

    let executor = RenderGraphExecutor::new();
    assert_eq!(
        executor.validate_with_device(&engine, &registry, &pool, &graph),
        Ok(())
    );
    let report = executor
        .execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None)
        .unwrap();
    assert_eq!(report.flattened_nodes, 3);
}

#[test]
fn execution_3_way_interleaved_nodes_are_ordered() {
    let mut builder = GpuEngineBuilder::new();
    builder = builder.with_required_limits(wgpu::Limits {
        max_compute_invocations_per_workgroup: 256,
        max_compute_workgroup_size_x: 256,
        max_compute_workgroup_size_y: 256,
        max_compute_workgroup_size_z: 64,
        ..Default::default()
    });
    let engine = pollster::block_on(builder.build()).unwrap();
    let mut registry = ResourceRegistry::new();

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }",
        )),
    });
    let pipeline_layout = engine
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            immediate_size: 0,
        });
    let draw_pipeline = engine
        .device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
        });
    registry.insert_pipeline_with_layout_descriptor(
        PipelineHandle(1),
        draw_pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![],
        },
    );

    let compute_shader = engine
        .device()
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@compute @workgroup_size(1) fn main() {}",
            )),
        });
    let compute_pipeline =
        engine
            .device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                module: &compute_shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
    registry.insert_compute_pipeline_with_layout_descriptor(
        ComputePipelineHandle(1),
        compute_pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![],
        },
    );

    let buffer1 = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let buffer2 = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(1),
            buffer1,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_SRC,
            },
        )
        .unwrap();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(2),
            buffer2,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_DST,
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
    )]);
    graph.add_node_id(id);
    let id = pool.alloc_copy_batch(vec![CopyCommand::buffer_to_buffer(
        BufferHandle(1),
        BufferHandle(2),
        4,
    )]);
    graph.add_node_id(id);
    let id = pool.alloc_compute_batch(vec![ComputeCommand::new(
        ComputePipelineHandle(1),
        [1, 1, 1],
    )]);
    graph.add_node_id(id);

    let executor = RenderGraphExecutor::new();
    assert_eq!(
        executor.validate_with_device(&engine, &registry, &pool, &graph),
        Ok(())
    );
    let report = executor
        .execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None)
        .unwrap();
    assert_eq!(report.draw_commands, 1);
    assert_eq!(report.copy_commands, 1);
    assert_eq!(report.compute_commands, 1);
    assert_eq!(report.flattened_nodes, 3);
}
