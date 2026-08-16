use super::RenderGraphExecutor;
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    ComputeCommand, CopyCommand, GraphResource, RenderGraph, RenderNodePool, RenderTarget,
    ResourceAccess,
};
use crate::resources::{
    BindGroupHandle, BindGroupResourceDescriptor, BufferHandle, BufferResourceDescriptor,
    ComputePipelineHandle, PipelineLayoutResourceDescriptor, ResourceRegistry,
};

#[test]
fn compute_only_graph_executes_storage_update_without_render_target() {
    let engine = pollster::block_on(
        GpuEngineBuilder::new()
            .with_required_limits(wgpu::Limits::default())
            .build(),
    )
    .unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compute_test"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
        )),
    });
    let layout = engine
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_test_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
    let pipeline_layout = engine
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_test_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
    let pipeline = engine
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_test_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute_test_buffer"),
        size: 4,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let staging = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("compute_test_staging"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    engine
        .queue()
        .write_buffer(&buffer, 0, bytemuck::bytes_of(&0u32));
    let bind_group = engine
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_test_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
    let mut registry = ResourceRegistry::new();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(1),
            buffer,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            },
        )
        .unwrap();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(2),
            staging,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            },
        )
        .unwrap();
    registry.insert_compute_pipeline_with_layout_descriptor(
        ComputePipelineHandle(1),
        pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![Some(1)],
        },
    );
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(1),
            bind_group,
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 1,
            },
        )
        .unwrap();
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_compute_batch(
        &mut pool,
        vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(
                0,
                BindGroupHandle(1),
                vec![],
            ),
        ],
    );
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(1),
            BufferHandle(2),
            4,
        )],
    );

    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut pool, &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission.clone()),
        timeout: None,
    });
    let staging = registry.buffer(&BufferHandle(2)).unwrap();
    let slice = staging.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    receiver.recv().unwrap().unwrap();
    let bytes = slice.get_mapped_range().unwrap();
    assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 1);
}

#[test]
fn flattened_execution_preserves_root_before_nested_compute_order() {
    let engine = pollster::block_on(
        GpuEngineBuilder::new()
            .with_required_limits(wgpu::Limits::default())
            .build(),
    )
    .unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("nested_order_compute"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
        )),
    });
    let layout = engine
        .device()
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nested_order_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
    let pipeline_layout = engine
        .device()
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nested_order_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
    let pipeline = engine
        .device()
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nested_order_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
    let make_buffer = |label, usage| {
        engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: 4,
            usage,
            mapped_at_creation: false,
        })
    };
    let source = make_buffer(
        "nested_order_source",
        wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    );
    let shared = make_buffer(
        "nested_order_shared",
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
    );
    let staging = make_buffer(
        "nested_order_staging",
        wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    );
    engine
        .queue()
        .write_buffer(&source, 0, bytemuck::bytes_of(&7u32));
    engine
        .queue()
        .write_buffer(&shared, 0, bytemuck::bytes_of(&0u32));
    let bind_group = engine
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nested_order_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shared.as_entire_binding(),
            }],
        });
    let mut registry = ResourceRegistry::new();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(1),
            source,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            },
        )
        .unwrap();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(2),
            shared,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST,
            },
        )
        .unwrap();
    registry
        .insert_buffer_with_descriptor(
            BufferHandle(3),
            staging,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            },
        )
        .unwrap();
    registry.insert_compute_pipeline_with_layout_descriptor(
        ComputePipelineHandle(1),
        pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: vec![Some(1)],
        },
    );
    registry
        .insert_bind_group_with_descriptor(
            BindGroupHandle(1),
            bind_group,
            BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 1,
            },
        )
        .unwrap();

    let mut pool = RenderNodePool::new();
    let mut child = RenderGraph::new(RenderTarget::Screen);
    let child_compute = child.add_compute_batch(
        &mut pool,
        vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(
                0,
                BindGroupHandle(1),
                vec![],
            ),
        ],
    );
    child.declare_resource_usage(
        child_compute,
        GraphResource::Buffer(BufferHandle(2)),
        ResourceAccess::ReadWrite,
    );
    let mut root = RenderGraph::new(RenderTarget::Screen);
    root.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(1),
            BufferHandle(2),
            4,
        )],
    );
    root.add_subgraph(&mut pool, "nested-order", child, vec![]);
    root.add_copy_batch(
        &mut pool,
        vec![CopyCommand::buffer_to_buffer(
            BufferHandle(2),
            BufferHandle(3),
            4,
        )],
    );

    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut pool, &root)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let staging = registry.buffer(&BufferHandle(3)).unwrap();
    let slice = staging.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    receiver.recv().unwrap().unwrap();
    let bytes = slice.get_mapped_range().unwrap();
    assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 8);
}
