use std::borrow::Cow;
use std::time::Instant;
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{
    BindGroupHandle, DrawAction, DrawCommand, MeshHandle, PipelineHandle, RenderGraph,
    RenderGraphExecutor, RenderTarget, ResourceRegistry, TextureHandle,
};

// Helper: Khởi tạo Texture làm bia vẽ (Target)
fn create_target(engine: &ifol_gpu::api::GpuEngine) -> (wgpu::TextureView, wgpu::Texture) {
    let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Target"),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
}

// Helper: Lưu ảnh
fn save_texture(engine: &ifol_gpu::api::GpuEngine, texture: &wgpu::Texture, filename: &str) {
    let path = std::path::Path::new("examples/outputs").join(filename);
    engine.save_texture_to_file(texture, &path).expect("Lỗi lưu ảnh");
}

fn test_01_clear_color(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.5, 0.8, 1.0]);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 01 (Clear Color) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_01_clear_color.png");
}

fn test_02_z_buffer(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"),
        size: wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    registry.textures.insert(
        TextureHandle(2),
        depth_tex.create_view(&wgpu::TextureViewDescriptor::default()),
    );

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            @vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32, @builtin(instance_index) in_instance_index: u32) -> VertexOutput {
                var out: VertexOutput;
                let x = f32(i32(in_vertex_index) - 1) * 0.5;
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                let offset_x = f32(in_instance_index) * 0.2 - 0.2;
                let offset_y = f32(in_instance_index) * 0.2 - 0.2;
                let z = 0.8 - f32(in_instance_index) * 0.3;
                out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
                if (in_instance_index == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); } 
                else if (in_instance_index == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); } 
                else { out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
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
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    registry.pipelines.insert(PipelineHandle(1), pipeline);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0])
    .with_depth_stencil(TextureHandle(2));

    let cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..3,
        },
    );
    graph.add_batch(vec![cmd]);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 02 (Z-Buffer) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_02_z_buffer.png");
}

fn test_03_alpha_blend(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            @vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32, @builtin(instance_index) in_instance_index: u32) -> VertexOutput {
                var out: VertexOutput;
                let x = f32(i32(in_vertex_index) - 1) * 0.5;
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                let offset_x = f32(in_instance_index) * 0.2 - 0.2;
                let offset_y = f32(in_instance_index) * 0.2 - 0.2;
                out.clip_position = vec4<f32>(x + offset_x, y + offset_y, 0.5, 1.0);
                if (in_instance_index == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 0.5); } 
                else if (in_instance_index == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 0.5); } 
                else { out.color = vec4<f32>(0.0, 0.0, 1.0, 0.5); } 
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
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
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

    registry.pipelines.insert(PipelineHandle(1), pipeline);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..3,
        },
    );
    graph.add_batch(vec![cmd]);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 03 (Alpha Blend) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_03_alpha_blend.png");
}

fn test_04_interleaved(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    use wgpu::util::DeviceExt;
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            
            @group(0) @binding(0)
            var<uniform> offset: vec2<f32>;

            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-0.2, -0.2), vec2<f32>(0.2, -0.2), vec2<f32>(-0.2, 0.2),
                    vec2<f32>(-0.2, 0.2), vec2<f32>(0.2, -0.2), vec2<f32>(0.2, 0.2)
                );
                var out: VertexOutput;
                out.clip_position = vec4<f32>(pos[id].x + offset.x, pos[id].y + offset.y, 0.5, 1.0);
                out.color = vec4<f32>(1.0, 1.0, 0.0, 0.5);
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
    });

    let bind_group_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
        label: None,
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let pipe_alpha = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
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
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

    let pipe_solid = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
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
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });

    registry.pipelines.insert(PipelineHandle(1), pipe_alpha);
    registry.pipelines.insert(PipelineHandle(2), pipe_solid);

    for i in 0..4 {
        let offset = [f32::from(i as i16) * 0.5 - 0.75, 0.0_f32];
        let buffer = engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&offset),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bg = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: None,
        });
        registry
            .bind_groups
            .insert(ifol_gpu::render::BindGroupHandle(i + 1), bg);
    }

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let commands = vec![
        DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 6,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, ifol_gpu::render::BindGroupHandle(1), vec![]),
        DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 6,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, ifol_gpu::render::BindGroupHandle(2), vec![]),
        DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 6,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, ifol_gpu::render::BindGroupHandle(3), vec![]),
        DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 6,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, ifol_gpu::render::BindGroupHandle(4), vec![]),
    ];
    graph.add_batch(commands);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 04 (Interleaved) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_04_interleaved.png");
}

fn test_05_garbage_collection(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(id) - 1) * 0.5; let y = f32(i32(id & 1u) * 2 - 1) * 0.5;
                return vec4(x, y, 0.5, 1.0);
            }
            @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4(1.0, 0.0, 0.0, 1.0); }
        ")),
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
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
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

    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.meshes.insert(
        MeshHandle(1),
        (
            engine.device().create_buffer(&wgpu::BufferDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::VERTEX,
                label: None,
                mapped_at_creation: false,
            }),
            None,
            3,
        ),
    );
    registry.meshes.insert(
        MeshHandle(2),
        (
            engine.device().create_buffer(&wgpu::BufferDescriptor {
                size: 4,
                usage: wgpu::BufferUsages::VERTEX,
                label: None,
                mapped_at_creation: false,
            }),
            None,
            3,
        ),
    );

    registry.remove_mesh(&MeshHandle(2));

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let commands = vec![
        DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Indexed {
                mesh: MeshHandle(2),
                index_range: 0..3,
                instance_range: 0..1,
            },
        ),
        DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Indexed {
                mesh: MeshHandle(1),
                index_range: 0..3,
                instance_range: 0..1,
            },
        ),
    ];
    graph.add_batch(commands);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 05 (Garbage Collection) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_05_garbage_collection.png");
}

fn test_07_complex_frame(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            
            fn rand(seed: u32) -> f32 {
                var s = seed;
                s = (s ^ 61u) ^ (s >> 16u);
                s = s * 9u;
                s = s ^ (s >> 4u);
                s = s * 668265261u;
                s = s ^ (s >> 15u);
                return f32(s) / 4294967296.0;
            }

            @vertex fn vs_main(@builtin(vertex_index) id: u32, @builtin(instance_index) inst: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
                    vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
                );

                var out: VertexOutput;
                let size = 0.005 + rand(inst * 13u) * 0.045;
                let pos_x = rand(inst * 19u) * 2.0 - 1.0;
                let pos_y = rand(inst * 23u) * 2.0 - 1.0;
                
                out.clip_position = vec4<f32>(pos[id].x * size + pos_x, pos[id].y * size + pos_y, 0.5, 1.0);
                
                let r = rand(inst * 31u);
                let g = rand(inst * 37u);
                let b = rand(inst * 41u);
                let a = 0.3 + rand(inst * 43u) * 0.7;
                out.color = vec4<f32>(r, g, b, a);
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
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
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

    registry.pipelines.insert(PipelineHandle(1), pipeline);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 6,
            instance_range: 0..50_000,
        },
    );
    graph.add_batch(vec![cmd]);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
    let duration = start.elapsed();

    println!("Test 07 (Complex Particles Quad) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_07_complex_frame.png");
}

fn test_08_multi_graph_cache(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(id) - 1) * 0.5; let y = f32(i32(id & 1u) * 2 - 1) * 0.5;
                return vec4(x, y, 0.5, 1.0);
            }
            @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4(0.0, 1.0, 0.0, 1.0); }
        ")),
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
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
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

    registry.pipelines.insert(PipelineHandle(1), pipeline);

    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let mut commands = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        commands.push(DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ));
    }
    graph.add_batch(commands);

    // Pass 1
    let start_1 = Instant::now();
    let idx_1 = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx_1),
        timeout: None,
    });
    let duration_1 = start_1.elapsed();

    // Pass 2
    let start_2 = Instant::now();
    let idx_2 = executor.execute(engine, &registry, &graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx_2),
        timeout: None,
    });
    let duration_2 = start_2.elapsed();

    println!("Test 08 (Multi Graph Cache) - Run 1: {:?}, Run 2: {:?}", duration_1, duration_2);
    save_texture(engine, &tex, "test_08_multi_graph.png");
}

fn test_09_subgraph_compositing(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();

    // Target chính (Root Target - Offscreen Texture 1)
    let (root_view, root_tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), root_view);

    // Target của SubGraph (Offscreen Texture 2 - 800x600)
    let (sub_view, _sub_tex) = create_target(engine);
    registry.textures.insert(TextureHandle(2), sub_view);

    // Shader vẽ Tam giác Đỏ trong SubGraph
    let inner_shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("InnerShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(id) - 1) * 0.8;
                let y = f32(i32(id & 1u) * 2 - 1) * 0.8;
                return vec4(x, y, 0.5, 1.0);
            }
            @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4(1.0, 0.2, 0.2, 1.0); }
        ")),
    });

    let layout_empty = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None, bind_group_layouts: &[], immediate_size: 0,
    });
    let inner_pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_empty),
        vertex: wgpu::VertexState { module: &inner_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &inner_shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(1), inner_pipeline);

    // Shader Composite ở Graph Cha (Lấy Texture 2 vẽ đè lên Target 1 với hiệu ứng Tint Xanh Lục)
    let composite_shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("CompositeShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @group(0) @binding(0) var src_tex: texture_2d<f32>;
            @group(0) @binding(1) var src_sampler: sampler;

            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };

            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var uvs = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
                var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                var out: VertexOutput;
                out.clip_position = vec4(pos[id], 0.0, 1.0);
                out.uv = uvs[id];
                return out;
            }

            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let color = textureSample(src_tex, src_sampler, in.uv);
                // Áp Shader Composite: Tint màu Xanh lá đè lên kết quả SubGraph
                return vec4(color.r * 0.2, color.g * 1.0, color.b * 0.5, color.a);
            }
        ")),
    });

    let bgl = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
        label: None,
    });
    let composite_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0,
    });
    let composite_pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&composite_layout),
        vertex: wgpu::VertexState { module: &composite_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &composite_shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(2), composite_pipeline);

    // BindGroup kết nối TextureHandle(2) (Offscreen của SubGraph) vào Shader Composite của Graph Cha
    let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor::default());
    let sub_view_ref = registry.textures.get(&TextureHandle(2)).unwrap();
    let composite_bg = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(sub_view_ref) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
        label: None,
    });
    registry.bind_groups.insert(BindGroupHandle(1), composite_bg);

    // DỰNG ĐỒ THỊ ĐỆ QUY SubGraph
    let mut inner_graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(2),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    inner_graph.add_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
    )]);

    let mut root_graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 800,
        height: 600,
    })
    .with_clear_color([0.05, 0.05, 0.05, 1.0]);

    let composite_cmd = DrawCommand::new(
        PipelineHandle(2),
        DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
    )
    .with_bind_group(0, BindGroupHandle(1), vec![]);

    // Nhét SubGraph vào RootGraph kèm composite_cmd
    root_graph.add_subgraph("CharacterSubGraph", inner_graph, vec![composite_cmd]);

    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &root_graph);
    let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();

    println!("Test 09 (SubGraph Compositing) Render Time: {:?}", duration);
    save_texture(engine, &root_tex, "test_09_subgraph_compositing.png");
}

fn test_10_ultimate_master_compositing(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    use wgpu::util::DeviceExt;
    let mut registry = ResourceRegistry::new();

    // 1. Tạo các Target Texture (Size 1024x1024)
    let create_tex = |label: &str| {
        let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
    };

    let (master_view, master_tex) = create_tex("MasterTarget");       // TextureHandle(1)
    let (effect_view, _effect_tex) = create_tex("EffectTarget");       // TextureHandle(2)
    let (char_view, _char_tex) = create_tex("CharTarget");             // TextureHandle(3)

    let char_depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("CharDepth"), size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });

    registry.textures.insert(TextureHandle(1), master_view);
    registry.textures.insert(TextureHandle(2), effect_view);
    registry.textures.insert(TextureHandle(3), char_view);
    registry.textures.insert(TextureHandle(4), char_depth_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    // Load ảnh thật (ai_demo_large.png) làm Background
    let ai_img_data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/ai_demo_large.png")).expect("Thiếu file ảnh demo");
    let img = image::load_from_memory(&ai_img_data).unwrap().to_rgba8();
    let dims = img.dimensions();

    let bg_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width: dims.0, height: dims.1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("RealImageBG"), view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo { texture: &bg_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &img,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * dims.0), rows_per_image: Some(dims.1) },
        wgpu::Extent3d { width: dims.0, height: dims.1, depth_or_array_layers: 1 },
    );
    registry.textures.insert(TextureHandle(5), bg_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    // Sampler chung
    let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let layout_empty = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    // Shader 1 (Lớp 3 - Innermost): 3D Z-Buffered Geometry
    let shader_3d = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("3DShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32, @builtin(instance_index) inst: u32) -> VertexOutput {
                var out: VertexOutput;
                let x = f32(i32(id) - 1) * 0.6;
                let y = f32(i32(id & 1u) * 2 - 1) * 0.6;
                let offset_x = f32(inst) * 0.25 - 0.25;
                let z = 0.8 - f32(inst) * 0.35;
                out.clip_position = vec4(x + offset_x, y, z, 1.0);
                if (inst == 0u) { out.color = vec4(1.0, 0.2, 0.2, 1.0); }
                else { out.color = vec4(0.2, 0.4, 1.0, 1.0); }
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
    });
    let pipe_3d = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_empty),
        vertex: wgpu::VertexState { module: &shader_3d, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_3d, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(),
        depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
        multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(1), pipe_3d);

    // Shader 2 (Lớp 2 - Middle): 10.000 Procedural Particles
    let shader_particles = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ParticleShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32> };
            fn rand(seed: u32) -> f32 {
                var s = seed * 1664525u + 1013904223u;
                return f32(s) / 4294967296.0;
            }
            @vertex fn vs_main(@builtin(vertex_index) id: u32, @builtin(instance_index) inst: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
                var out: VertexOutput;
                let size = 0.004 + rand(inst * 7u) * 0.015;
                let px = rand(inst * 13u) * 2.0 - 1.0;
                let py = rand(inst * 17u) * 2.0 - 1.0;
                out.clip_position = vec4(pos[id].x * size + px, pos[id].y * size + py, 0.1, 1.0);
                out.color = vec4(0.9, 0.8, 0.2, 0.6); // Vàng sáng lấp lánh 60%
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
    });
    let pipe_particles = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_empty),
        vertex: wgpu::VertexState { module: &shader_particles, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_particles, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(2), pipe_particles);

    // BindGroupLayout & Shader cho Composite Texture (Sampler + Texture)
    let bgl_tex = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
        label: None,
    });
    let layout_tex = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl_tex)], immediate_size: 0 });

    // Composite Shader 1: Lấy CharTarget (3) vẽ đè lên EffectTarget (2) kèm Neon Tint
    let shader_comp_char = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("CompCharShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @group(0) @binding(0) var tex: texture_2d<f32>;
            @group(0) @binding(1) var smp: sampler;
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var uvs = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
                var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                var out: VertexOutput;
                out.clip_position = vec4(pos[id], 0.0, 1.0);
                out.uv = uvs[id];
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let c = textureSample(tex, smp, in.uv);
                return vec4(c.r * 0.4 + 0.1, c.g * 1.2, c.b * 1.5, c.a);
            }
        ")),
    });
    let pipe_comp_char = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_tex),
        vertex: wgpu::VertexState { module: &shader_comp_char, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_comp_char, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(3), pipe_comp_char);

    // Composite Shader 2 (Lớp Master): Lấy Ảnh Thật BG (5) làm nền + Lấy EffectTarget (2) đè lên với Vignette Post-FX
    let shader_final_post = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FinalPostShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @group(0) @binding(0) var tex: texture_2d<f32>;
            @group(0) @binding(1) var smp: sampler;
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var uvs = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
                var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                var out: VertexOutput;
                out.clip_position = vec4(pos[id], 0.0, 1.0);
                out.uv = uvs[id];
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let c = textureSample(tex, smp, in.uv);
                let dist = distance(in.uv, vec2(0.5, 0.5));
                let vignette = smoothstep(0.8, 0.2, dist);
                return vec4(c.rgb * vignette, c.a);
            }
        ")),
    });
    let pipe_final_post = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_tex),
        vertex: wgpu::VertexState { module: &shader_final_post, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_final_post, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(4), pipe_final_post);

    // Bind Groups
    let make_bg = |handle_id: u64, tex_handle: TextureHandle| {
        let view_ref = registry.textures.get(&tex_handle).unwrap();
        engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view_ref) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
            label: None,
        })
    };

    let bg_char = make_bg(1, TextureHandle(3));  // Texture 3 (CharTarget)
    let bg_effect = make_bg(2, TextureHandle(2)); // Texture 2 (EffectTarget)
    let bg_real_img = make_bg(3, TextureHandle(5)); // Texture 5 (RealImageBG)

    registry.bind_groups.insert(BindGroupHandle(1), bg_char);
    registry.bind_groups.insert(BindGroupHandle(2), bg_effect);
    registry.bind_groups.insert(BindGroupHandle(3), bg_real_img);

    // DỰNG ĐỒ THỊ ĐỆ QUY 3 CẤP ĐỘ (3-LEVEL RECURSIVE RENDER GRAPH)
    
    // Lớp 3 (Sâu nhất): Character Graph (Offscreen Texture 3 + Depth Texture 4)
    let mut char_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0])
        .with_depth_stencil(TextureHandle(4));
    char_graph.add_batch(vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural { vertex_count: 3, instance_range: 0..2 },
    )]);

    // Lớp 2 (Trung gian): Effect Graph (Offscreen Texture 2)
    let mut effect_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(2), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
    
    // Nhét Lớp 3 vào Lớp 2 kèm Composite Command đọc Texture 3
    let comp_char_cmd = DrawCommand::new(PipelineHandle(3), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(1), vec![]);
    effect_graph.add_subgraph("CharacterSubGraph", char_graph, vec![comp_char_cmd]);

    // Thêm Lớp Hạt Bụi 10.000 hạt vào Effect Graph
    effect_graph.add_batch(vec![DrawCommand::new(
        PipelineHandle(2),
        DrawAction::Procedural { vertex_count: 6, instance_range: 0..10_000 },
    )]);

    // Lớp 1 (Root Master): Master Scene Graph (Target: Master Texture 1)
    let mut master_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(1), width: 1024, height: 1024 })
        .with_clear_color([0.02, 0.02, 0.04, 1.0]);

    // 1. In Ảnh Thật (Real Image BG) làm Nền
    let draw_bg_cmd = DrawCommand::new(PipelineHandle(4), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(3), vec![]);
    master_graph.add_batch(vec![draw_bg_cmd]);

    // 2. Nhét Lớp 2 (Effect Graph) vào Master Graph kèm Post-FX Vignette Command đọc Texture 2
    let comp_final_cmd = DrawCommand::new(PipelineHandle(4), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(2), vec![]);
    master_graph.add_subgraph("EffectAndCharSubGraph", effect_graph, vec![comp_final_cmd]);

    // THỰC THI BIÊN DỊCH VÀ VẼ
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &master_graph);
    let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();

    println!("Test 10 (Ultimate Master 3-Level Compositing) Render Time: {:?}", duration);
    save_texture(engine, &master_tex, "test_10_ultimate_master.png");
}

fn test_11_extreme_motion_graphics_pipeline(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();

    // 1. Tạo các Target Texture (Size 1024x1024)
    let create_tex = |label: &str| {
        let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
    };

    let (master_view, master_tex) = create_tex("MasterTarget");       // TextureHandle(1)
    let (comp_view, _comp_tex) = create_tex("CompositeTarget");       // TextureHandle(2)
    let (char_view, _char_tex) = create_tex("CharTarget");             // TextureHandle(3)
    let (blur_h_view, _blur_h_tex) = create_tex("BlurHTarget");       // TextureHandle(6)
    let (blur_v_view, _blur_v_tex) = create_tex("BlurVTarget");       // TextureHandle(7)
    let (particle_view, _particle_tex) = create_tex("ParticleTarget");// TextureHandle(8)

    let char_depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("CharDepth"), size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });

    registry.textures.insert(TextureHandle(1), master_view);
    registry.textures.insert(TextureHandle(2), comp_view);
    registry.textures.insert(TextureHandle(3), char_view);
    registry.textures.insert(TextureHandle(4), char_depth_tex.create_view(&wgpu::TextureViewDescriptor::default()));
    registry.textures.insert(TextureHandle(6), blur_h_view);
    registry.textures.insert(TextureHandle(7), blur_v_view);
    registry.textures.insert(TextureHandle(8), particle_view);

    // Load ảnh thật (ai_demo_large.png) làm Background
    let ai_img_data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/assets/ai_demo_large.png")).expect("Thiếu file ảnh demo");
    let img = image::load_from_memory(&ai_img_data).unwrap().to_rgba8();
    let dims = img.dimensions();

    let bg_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d { width: dims.0, height: dims.1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("RealImageBG"), view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo { texture: &bg_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &img,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * dims.0), rows_per_image: Some(dims.1) },
        wgpu::Extent3d { width: dims.0, height: dims.1, depth_or_array_layers: 1 },
    );
    registry.textures.insert(TextureHandle(5), bg_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default()
    });

    let layout_empty = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    // SHADER VẬT THỂ 1: Metallic Mesh Shader (Màu Kim loại ánh Bạc)
    let shader_obj1 = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("MetallicShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 3>(vec2(-0.8, -0.6), vec2(-0.1, -0.6), vec2(-0.45, 0.7));
                var out: VertexOutput; out.clip_position = vec4(pos[id], 0.3, 1.0); out.uv = pos[id]; return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let stripe = sin(in.uv.y * 40.0) * 0.3 + 0.7;
                return vec4(0.8 * stripe, 0.85 * stripe, 0.95 * stripe, 1.0);
            }
        ")),
    });

    // SHADER VẬT THỂ 2: Fire Lava Shader (Màu Lửa Đỏ Cam Hỏa Ngục)
    let shader_obj2 = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FireShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 3>(vec2(0.1, -0.6), vec2(0.8, -0.6), vec2(0.45, 0.7));
                var out: VertexOutput; out.clip_position = vec4(pos[id], 0.5, 1.0); out.uv = pos[id]; return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let pulse = sin(in.uv.x * 20.0 + in.uv.y * 30.0) * 0.4 + 0.6;
                return vec4(1.0 * pulse, 0.3 * pulse, 0.05, 1.0);
            }
        ")),
    });

    // SHADER VẬT THỂ 3: Hologram Scanline Shader (Xanh Neon)
    let shader_obj3 = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HologramShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 3>(vec2(-0.4, -0.2), vec2(0.4, -0.2), vec2(0.0, 0.9));
                var out: VertexOutput; out.clip_position = vec4(pos[id], 0.2, 1.0); out.uv = pos[id]; return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let scanline = step(0.5, fract(in.uv.y * 50.0));
                return vec4(0.0, 0.9 * scanline + 0.1, 1.0, 0.85);
            }
        ")),
    });

    let depth_state = Some(wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true),
        depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default(),
    });

    let make_obj_pipe = |shader: &wgpu::ShaderModule| engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_empty),
        vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: depth_state.clone(), multisample: Default::default(), multiview_mask: None, cache: None,
    });

    registry.pipelines.insert(PipelineHandle(1), make_obj_pipe(&shader_obj1));
    registry.pipelines.insert(PipelineHandle(2), make_obj_pipe(&shader_obj2));
    registry.pipelines.insert(PipelineHandle(3), make_obj_pipe(&shader_obj3));

    // SHADER HẠT BỤI SPARKLES (25.000 Hạt)
    let shader_sparks = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("SparklesShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32> };
            fn rand(seed: u32) -> f32 {
                var s = seed * 1664525u + 1013904223u; return f32(s) / 4294967296.0;
            }
            @vertex fn vs_main(@builtin(vertex_index) id: u32, @builtin(instance_index) inst: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0));
                var out: VertexOutput;
                let size = 0.003 + rand(inst * 5u) * 0.012;
                let px = rand(inst * 11u) * 2.0 - 1.0;
                let py = rand(inst * 19u) * 2.0 - 1.0;
                out.clip_position = vec4(pos[id].x * size + px, pos[id].y * size + py, 0.1, 1.0);
                out.color = vec4(0.2, 1.0, 0.8, 0.8); // Cyan Sparkles
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        ")),
    });
    let pipe_sparks = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_empty),
        vertex: wgpu::VertexState { module: &shader_sparks, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_sparks, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(5), pipe_sparks);

    // BindGroupLayout & Layout cho Texture Processing Shaders
    let bgl_tex = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ], label: None,
    });
    let layout_tex = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl_tex)], immediate_size: 0 });

    // SHADER BOX BLUR HORIZONTAL & VERTICAL (Tạo hiệu ứng Glow / Bloom mờ ảo)
    let shader_blur = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("BoxBlurShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @group(0) @binding(0) var tex: texture_2d<f32>;
            @group(0) @binding(1) var smp: sampler;
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var uvs = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
                var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                var out: VertexOutput; out.clip_position = vec4(pos[id], 0.0, 1.0); out.uv = uvs[id]; return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                var col = vec4(0.0);
                let off = 0.006;
                // Box Filter 9 mẫu làm mờ mượt mà
                for (var x: i32 = -2; x <= 2; x++) {
                    for (var y: i32 = -2; y <= 2; y++) {
                        col += textureSample(tex, smp, in.uv + vec2(f32(x) * off, f32(y) * off));
                    }
                }
                return col / 25.0 * 1.5; // Tăng cường độ Glow Bloom
            }
        ")),
    });
    let pipe_blur = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_tex),
        vertex: wgpu::VertexState { module: &shader_blur, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_blur, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(6), pipe_blur);

    // MASTER SHADER POST-FX: RGB Split Chromatic Aberration Glitch + Film Grain + Scanline
    let shader_glitch = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("GlitchPostShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @group(0) @binding(0) var main_tex: texture_2d<f32>;
            @group(0) @binding(1) var smp: sampler;

            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32> };
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var uvs = array<vec2<f32>, 3>(vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
                var pos = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
                var out: VertexOutput; out.clip_position = vec4(pos[id], 0.0, 1.0); out.uv = uvs[id]; return out;
            }

            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
                let uv = in.uv;
                
                // 1. Phân tách kênh màu RGB (Chromatic Aberration RGB Split Glitch)
                let r_offset = vec2<f32>(0.009, 0.003);
                let b_offset = vec2<f32>(-0.009, -0.003);
                
                let r = textureSample(main_tex, smp, uv + r_offset).r;
                let g = textureSample(main_tex, smp, uv).g;
                let b = textureSample(main_tex, smp, uv + b_offset).b;
                let a = textureSample(main_tex, smp, uv).a;

                var color = vec3<f32>(r, g, b);

                // 2. Scanline Glitch sọc kẻ nằm ngang
                let scanline = sin(uv.y * 300.0) * 0.08;
                color -= scanline;

                // 3. Vignette tối 4 góc
                let dist = distance(uv, vec2<f32>(0.5, 0.5));
                let vignette = smoothstep(0.85, 0.25, dist);
                color *= vignette;

                return vec4<f32>(color, a);
            }
        ")),
    });

    let pipe_glitch = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout_tex),
        vertex: wgpu::VertexState { module: &shader_glitch, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader_glitch, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(8), pipe_glitch);

    // Bind Groups
    let make_bg = |tex_handle: TextureHandle| {
        let view_ref = registry.textures.get(&tex_handle).unwrap();
        engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bgl_tex,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view_ref) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ], label: None,
        })
    };

    registry.bind_groups.insert(BindGroupHandle(1), make_bg(TextureHandle(3))); // CharTarget (3)
    registry.bind_groups.insert(BindGroupHandle(2), make_bg(TextureHandle(7))); // BlurTarget (7)
    registry.bind_groups.insert(BindGroupHandle(3), make_bg(TextureHandle(8))); // ParticleTarget (8)
    registry.bind_groups.insert(BindGroupHandle(4), make_bg(TextureHandle(2))); // CompositeTarget (2)
    registry.bind_groups.insert(BindGroupHandle(5), make_bg(TextureHandle(5))); // RealImageBG (5)

    // =========================================================================
    // XÂY DỰNG RENDER GRAPH AAA MOTION GRAPHICS COMPLEX PIPELINE
    // =========================================================================

    // 1. SubGraph Lớp Đáy: CharGraph (Offscreen Texture 3 + Depth 4)
    // Chứa 3 vật thể 3D vẽ bằng 3 Shader khác nhau hoàn toàn!
    let mut char_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0])
        .with_depth_stencil(TextureHandle(4));
    char_graph.add_batch(vec![
        DrawCommand::new(PipelineHandle(1), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 }), // Object 1: Metallic
        DrawCommand::new(PipelineHandle(2), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 }), // Object 2: Fire Lava
        DrawCommand::new(PipelineHandle(3), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 }), // Object 3: Hologram
    ]);

    // 2. SubGraph Lọc Mờ: BlurGraph (Offscreen Texture 7)
    // Nhận Texture 3 làm mờ Box Blur tạo hiệu ứng Glow Bloom
    let mut blur_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(7), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
    let blur_cmd = DrawCommand::new(PipelineHandle(6), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(1), vec![]); // Đọc CharTarget (3)
    blur_graph.add_batch(vec![blur_cmd]);

    // 3. SubGraph Hạt: ParticleGraph (Offscreen Texture 8)
    // 25.000 Hạt Sparkles Cyan
    let mut particle_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(8), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
    particle_graph.add_batch(vec![
        DrawCommand::new(PipelineHandle(5), DrawAction::Procedural { vertex_count: 6, instance_range: 0..25_000 })
    ]);

    // 4. SubGraph Hòa Trộn Lớp: CompositeGraph (Offscreen Texture 2)
    // Gom CharGraph + BlurGraph + ParticleGraph vào 1 Target
    let mut comp_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(2), width: 1024, height: 1024 })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);

    // Lệnh 1: In Lớp Glow Bloom (Blur 7) bên dưới
    let draw_glow = DrawCommand::new(PipelineHandle(6), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(2), vec![]);
    // Lệnh 2: In Lớp Vật Thể 3D Sắc Nét (Char 3) đè lên
    let draw_char = DrawCommand::new(PipelineHandle(6), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(1), vec![]);
    // Lệnh 3: In Lớp Hạt Sparkles (Particle 8) phủ trên cùng
    let draw_sparks = DrawCommand::new(PipelineHandle(6), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(3), vec![]);

    // Nhét 3 SubGraph con vào CompositeGraph
    comp_graph.add_subgraph("CharGraph", char_graph, vec![draw_char]);
    comp_graph.add_subgraph("BlurGraph", blur_graph, vec![draw_glow]);
    comp_graph.add_subgraph("ParticleGraph", particle_graph, vec![draw_sparks]);

    // 5. Root Master Graph (Master Target 1)
    // Vẽ Ảnh thật Cyberpunk BG + Áp Shader Glitch RGB Split trên toàn bộ Frame
    let mut master_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(1), width: 1024, height: 1024 })
        .with_clear_color([0.01, 0.01, 0.02, 1.0]);

    // Lệnh 1: In Ảnh Thật BG
    let draw_bg = DrawCommand::new(PipelineHandle(6), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(5), vec![]);
    master_graph.add_batch(vec![draw_bg]);

    // Lệnh 2: Nhét CompositeGraph vào MasterGraph kèm Shader RGB Split Glitch Post-FX
    let draw_glitch = DrawCommand::new(PipelineHandle(8), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(4), vec![]);
    master_graph.add_subgraph("MasterCompositePipeline", comp_graph, vec![draw_glitch]);

    // THỰC THI BIÊN DỊCH VÀ VẼ
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &master_graph);
    let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();

    println!("Test 11 (Extreme Motion Graphics AAA Pipeline) Render Time: {:?}", duration);
    save_texture(engine, &master_tex, "test_11_extreme_motion_graphics.png");
}

fn main() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    let executor = RenderGraphExecutor::new();

    std::fs::create_dir_all("examples/outputs").unwrap();

    println!("Running Test 01..."); test_01_clear_color(&engine, &executor);
    println!("Running Test 02..."); test_02_z_buffer(&engine, &executor);
    println!("Running Test 03..."); test_03_alpha_blend(&engine, &executor);
    println!("Running Test 04..."); test_04_interleaved(&engine, &executor);
    println!("Running Test 05..."); test_05_garbage_collection(&engine, &executor);
    println!("Running Test 07..."); test_07_complex_frame(&engine, &executor);
    println!("Running Test 08..."); test_08_multi_graph_cache(&engine, &executor);
    println!("Running Test 09..."); test_09_subgraph_compositing(&engine, &executor);
    println!("Running Test 10..."); test_10_ultimate_master_compositing(&engine, &executor);
    println!("Running Test 11..."); test_11_extreme_motion_graphics_pipeline(&engine, &executor);

    println!("Tất cả các bài test đã sinh ảnh thành công trong thư mục examples/outputs/");
}


