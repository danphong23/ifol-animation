use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{
    CopyCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use crate::resources::{
    BufferHandle, BufferResourceDescriptor, PipelineHandle, PipelineLayoutResourceDescriptor,
    ResourceRegistry, TextureHandle, TextureResourceDescriptor,
};

#[test]
fn copy_only_graph_executes_without_render_target() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let source = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("copy_source"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let destination = engine.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("copy_destination"),
        size: 4,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    engine.queue().write_buffer(&source, 0, &[7, 8, 9, 10]);

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
            destination,
            BufferResourceDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
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

    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut pool, &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission.clone()),
        timeout: None,
    });
    let destination = registry.buffer(&BufferHandle(2)).unwrap();
    let slice = destination.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    receiver.recv().unwrap().unwrap();
    assert_eq!(&*slice.get_mapped_range().unwrap(), &[7, 8, 9, 10]);
}

#[test]
fn texture_copy_graph_executes_and_preserves_pixels() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
    let descriptor = TextureResourceDescriptor {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        mip_level_count: 1,
        sample_count: 1,
    };
    let create_texture = |label| {
        engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    };
    let source = create_texture("texture_copy_source");
    let destination = create_texture("texture_copy_destination");
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    engine.queue().write_texture(
        source.as_image_copy(),
        &pixels,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(8),
            rows_per_image: Some(2),
        },
        wgpu::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
    );

    let mut registry = ResourceRegistry::new();
    registry
        .insert_owned_texture(TextureHandle(1), source, descriptor, 1024)
        .unwrap();
    registry
        .insert_owned_texture(TextureHandle(2), destination, descriptor, 1024)
        .unwrap();
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::texture_to_texture(
            TextureHandle(1),
            TextureHandle(2),
            [2, 2, 1],
        )],
    );

    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut pool, &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let readback = engine
        .read_texture_to_raw_with_format_checked(
            registry.owned_texture(&TextureHandle(2)).unwrap(),
            wgpu::TextureFormat::Rgba8Unorm,
        )
        .unwrap();
    assert_eq!((readback.width, readback.height), (2, 2));
    assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(readback.bytes, pixels);
}

#[test]
fn texture_copy_validation_rejects_missing_ownership_and_out_of_bounds_extent() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
    let descriptor = TextureResourceDescriptor {
        width: 4,
        height: 4,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        mip_level_count: 1,
        sample_count: 1,
    };
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("validation_texture"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    let mut registry = ResourceRegistry::new();
    registry
        .insert_owned_texture(TextureHandle(1), texture, descriptor, 1024)
        .unwrap();
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::texture_to_texture(
            TextureHandle(1),
            TextureHandle(2),
            [1, 1, 1],
        )],
    );
    assert_eq!(
        RenderGraphExecutor::new().validate(&registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingTexture(TextureHandle(2)))
    );

    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("validation_texture_destination"),
        size: wgpu::Extent3d {
            width: 4,
            height: 4,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        view_formats: &[],
    });
    registry
        .insert_owned_texture(TextureHandle(2), texture, descriptor, 1024)
        .unwrap();
    graph = RenderGraph::new(RenderTarget::Screen);
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::texture_to_texture(
            TextureHandle(1),
            TextureHandle(2),
            [5, 3, 1],
        )],
    );
    assert!(matches!(
        RenderGraphExecutor::new().validate(&registry, &pool, &graph),
        Err(RenderGraphValidationError::InvalidTextureCopyRange { .. })
    ));
}

#[test]
fn target_graph_with_interleaved_copy_and_draw_uses_ordered_segments() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ordered_segments_shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
            "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)); return vec4<f32>(p[i], 0.0, 1.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }",
        )),
    });
    let pipeline = engine
        .device()
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ordered_segments_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::COPY_SRC
        | wgpu::TextureUsages::COPY_DST;
    let descriptor = TextureResourceDescriptor {
        width: 2,
        height: 2,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage,
        mip_level_count: 1,
        sample_count: 1,
    };
    let make_texture = |label| {
        engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 2,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    };
    let mut registry = ResourceRegistry::new();
    registry
        .insert_owned_texture(
            TextureHandle(1),
            make_texture("ordered_source"),
            descriptor,
            1024,
        )
        .unwrap();
    registry
        .insert_owned_texture(
            TextureHandle(2),
            make_texture("ordered_copy_destination"),
            descriptor,
            1024,
        )
        .unwrap();
    registry
        .insert_owned_texture(
            TextureHandle(3),
            make_texture("ordered_target"),
            descriptor,
            1024,
        )
        .unwrap();
    registry.insert_pipeline_with_layout_descriptor(
        PipelineHandle(1),
        pipeline,
        PipelineLayoutResourceDescriptor {
            bind_group_layout_signatures: Vec::new(),
        },
    );
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(3),
        width: 2,
        height: 2,
    });
    graph.add_copy_batch(
        &mut pool,
        vec![CopyCommand::texture_to_texture(
            TextureHandle(1),
            TextureHandle(2),
            [2, 2, 1],
        )],
    );
    graph.add_batch(
        &mut pool,
        vec![DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        )],
    );
    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut pool, &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let readback = engine
        .read_texture_to_raw_with_format_checked(
            registry.owned_texture(&TextureHandle(3)).unwrap(),
            wgpu::TextureFormat::Rgba8Unorm,
        )
        .unwrap();
    assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
    assert_eq!(&readback.bytes[0..4], &[255, 0, 0, 255]);
}
