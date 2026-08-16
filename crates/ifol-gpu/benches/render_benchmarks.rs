use criterion::{criterion_group, criterion_main, Criterion};
use ifol_gpu::backend::GpuEngineBuilder;
use ifol_gpu::execution::RenderGraphExecutor;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use ifol_gpu::resources::{
    BindGroupHandle, BindGroupResourceDescriptor, PipelineHandle,
    PipelineLayoutResourceDescriptor, ResourceRegistry, TextureHandle,
    TextureResourceDescriptor,
};
use std::borrow::Cow;

fn register_color_texture(
    registry: &mut ResourceRegistry,
    handle: TextureHandle,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
) {
    registry.insert_texture_with_descriptor(
        handle,
        view,
        TextureResourceDescriptor {
            width,
            height,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        },
        16384,
    ).unwrap();
}

fn register_depth_texture(
    registry: &mut ResourceRegistry,
    handle: TextureHandle,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
) {
    registry.insert_texture_with_descriptor(
        handle,
        view,
        TextureResourceDescriptor {
            width,
            height,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            mip_level_count: 1,
            sample_count: 1,
        },
        16384,
    ).unwrap();
}

fn register_pipeline(
    registry: &mut ResourceRegistry,
    handle: PipelineHandle,
    pipeline: wgpu::RenderPipeline,
) {
    register_pipeline_with_layout_descriptor(
        registry,
        handle,
        pipeline,
        PipelineLayoutResourceDescriptor { bind_group_layout_signatures: Vec::new() },
    );
}

fn register_pipeline_with_layout_descriptor(
    registry: &mut ResourceRegistry,
    handle: PipelineHandle,
    pipeline: wgpu::RenderPipeline,
    descriptor: PipelineLayoutResourceDescriptor,
) {
    registry.insert_pipeline_with_layout_descriptor(
        handle,
        pipeline,
        descriptor,
    );
}

fn register_bind_group(
    registry: &mut ResourceRegistry,
    handle: BindGroupHandle,
    bind_group: wgpu::BindGroup,
) {
    registry.insert_bind_group_with_descriptor(
        handle,
        bind_group,
        BindGroupResourceDescriptor {
            dynamic_offset_count: 0,
            dynamic_offset_alignment: 0,
            layout_signature: 0,
        },
    ).unwrap();
}

fn bench_clear_screen(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let mut registry = ResourceRegistry::new();
    let tex_handle = TextureHandle(1);
    register_color_texture(&mut registry, tex_handle, target_view, 800, 600);

    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: tex_handle,
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    c.bench_function("bench_clear_screen", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_empty_graph(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let registry = ResourceRegistry::new();
    let graph = RenderGraph::new(RenderTarget::Screen);

    c.bench_function("bench_empty_graph", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_complex_graph(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    register_color_texture(
        &mut registry,
        TextureHandle(1),
        target_tex.create_view(&wgpu::TextureViewDescriptor::default()),
        1,
        1,
    );

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )),
    });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    register_pipeline(&mut registry, PipelineHandle(1), pipeline.clone());
    register_pipeline(&mut registry, PipelineHandle(2), pipeline);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });

    for _ in 0..100 {
        let mut commands = Vec::with_capacity(100);
        for j in 0..100 {
            let pipe = if j % 2 == 0 {
                PipelineHandle(1)
            } else {
                PipelineHandle(2)
            };
            commands.push(DrawCommand::new(
                pipe,
                DrawAction::Procedural {
                    vertex_count: 3,
                    instance_range: 0..1,
                },
            ));
        }
        graph.add_batch(&mut pool, commands);
    }

    c.bench_function("bench_complex_graph_100_nodes", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_single_large_image(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Target"),
        size: wgpu::Extent3d {
            width: 1024,
            height: 1024,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let shader_src = std::fs::read_to_string("benches/assets/basic_texture.wgsl").unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_src)),
    });

    let ai_img_data = std::fs::read("benches/assets/ai_demo_large.png").unwrap();
    let img = image::load_from_memory(&ai_img_data).unwrap().to_rgba8();
    let dims = img.dimensions();

    let texture_size = wgpu::Extent3d {
        width: dims.0,
        height: dims.1,
        depth_or_array_layers: 1,
    };
    let src_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("AI Large Image"),
        view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &src_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dims.0),
            rows_per_image: Some(dims.1),
        },
        texture_size,
    );
    let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bgl = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: None,
    });
    let bg = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&src_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: None,
    });

    let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    let mut registry = ResourceRegistry::new();
    register_color_texture(&mut registry, TextureHandle(1), target_view, 1024, 1024);
    register_pipeline_with_layout_descriptor(
        &mut registry,
        PipelineHandle(1),
        pipeline,
        PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(0)] },
    );
    register_bind_group(&mut registry, BindGroupHandle(1), bg);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1024,
        height: 1024,
    });
    let cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..1,
        },
    )
    .with_bind_group(0, BindGroupHandle(1), vec![]);

    graph.add_batch(&mut pool, vec![cmd]);

    c.bench_function("bench_single_large_image", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_100k_sprites_cpu_stress(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    register_pipeline(
        &mut registry,
        PipelineHandle(1),
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Dummy"),
            layout: Some(&engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                immediate_size: 0,
            })),
            vertex: wgpu::VertexState {
                module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                        "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }",
                    )),
                }),
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                        "@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
                    )),
                }),
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        }),
    );

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
    register_color_texture(&mut registry, TextureHandle(1), target_view, 1, 1);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });

    let mut commands = Vec::with_capacity(100_000);
    for _ in 0..100_000 {
        commands.push(DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
    }
    graph.add_batch(&mut pool, commands);

    c.bench_function("bench_100k_sprites_cpu_stress", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_100k_sprites_gpu_instanced(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    register_pipeline(
        &mut registry,
        PipelineHandle(1),
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Dummy"),
            layout: Some(&engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                immediate_size: 0,
            })),
            vertex: wgpu::VertexState {
                module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                        "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }",
                    )),
                }),
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
                        "@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
                    )),
                }),
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        }),
    );

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
    register_color_texture(&mut registry, TextureHandle(1), target_view, 1, 1);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });

    let cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..100_000,
        },
    );
    graph.add_batch(&mut pool, vec![cmd]);

    c.bench_function("bench_100k_sprites_gpu_instanced", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_z_buffer(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DepthTexture"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    register_color_texture(&mut registry, TextureHandle(1), target_view, 1, 1);
    register_depth_texture(&mut registry, TextureHandle(2), depth_view, 1, 1);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )),
    });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let create_pipeline = |depth: bool| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: if depth {
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                })
            } else {
                None
            },
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    };

    register_pipeline(&mut registry, PipelineHandle(1), create_pipeline(false));
    register_pipeline(&mut registry, PipelineHandle(2), create_pipeline(true));

    let mut graph_no_depth = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });
    graph_no_depth.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..10000,
        },
    )]);

    let mut graph_with_depth = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    })
    .with_depth_stencil(TextureHandle(2));
    graph_with_depth.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(2),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..10000,
        },
    )]);

    c.bench_function("bench_z_buffer_disabled", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_no_depth).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
    c.bench_function("bench_z_buffer_enabled", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_with_depth).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_alpha_blending(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    register_color_texture(
        &mut registry,
        TextureHandle(1),
        target_tex.create_view(&wgpu::TextureViewDescriptor::default()),
        1,
        1,
    );

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )),
    });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let create_pipeline = |blend: Option<wgpu::BlendState>| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        })
    };

    register_pipeline(&mut registry, PipelineHandle(1), create_pipeline(Some(wgpu::BlendState::REPLACE)));
    register_pipeline(&mut registry, PipelineHandle(2), create_pipeline(Some(wgpu::BlendState::ALPHA_BLENDING)));

    let mut graph_replace = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });
    graph_replace.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..10000,
        },
    )]);

    let mut graph_alpha = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });
    graph_alpha.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(2),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..10000,
        },
    )]);

    c.bench_function("bench_alpha_blend_replace", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_replace).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
    c.bench_function("bench_alpha_blend_alpha", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_alpha).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

fn bench_pipeline_caching(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    register_color_texture(
        &mut registry,
        TextureHandle(1),
        target_tex.create_view(&wgpu::TextureViewDescriptor::default()),
        1,
        1,
    );

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(
            "@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }",
        )),
    });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    register_pipeline(&mut registry, PipelineHandle(1), pipeline.clone());
    register_pipeline(&mut registry, PipelineHandle(2), pipeline);

    let mut graph_sorted = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });
    let mut sorted_commands = Vec::with_capacity(10000);
    for _ in 0..5000 {
        sorted_commands.push(DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
    }
    for _ in 0..5000 {
        sorted_commands.push(DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
    }
    graph_sorted.add_batch(&mut pool, sorted_commands);

    let mut graph_unsorted = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1,
        height: 1,
    });
    let mut unsorted_commands = Vec::with_capacity(10000);
    for _ in 0..5000 {
        unsorted_commands.push(DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
        unsorted_commands.push(DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
    }
    graph_unsorted.add_batch(&mut pool, unsorted_commands);

    c.bench_function("bench_pipeline_state_sorted", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_sorted).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
    c.bench_function("bench_pipeline_state_unsorted", |b| {
        b.iter(|| {
            let idx = executor.execute_checked(&engine, &registry, &mut pool, &graph_unsorted).expect("benchmark graph must validate");
            let _ = engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            });
        })
    });
}

criterion_group!(
    benches,
    bench_clear_screen,
    bench_empty_graph,
    bench_complex_graph,
    bench_single_large_image,
    bench_100k_sprites_cpu_stress,
    bench_100k_sprites_gpu_instanced,
    bench_z_buffer,
    bench_alpha_blending,
    bench_pipeline_caching
);
criterion_main!(benches);
