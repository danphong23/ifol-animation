use std::borrow::Cow;
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{
    DrawAction, DrawCommand, PipelineHandle, RenderGraph, RenderGraphExecutor, RenderTarget,
    ResourceRegistry, TextureHandle,
};

fn main() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    let executor = RenderGraphExecutor::new();

    let width = 800;
    let height = 600;

    let create_target = || {
        let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
    };

    let (z_target_view, z_target_tex) = create_target();
    let (alpha_target_view, alpha_target_tex) = create_target();

    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"),
        size: wgpu::Extent3d {
            width,
            height,
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

    let mut registry = ResourceRegistry::new();
    registry.textures.insert(TextureHandle(1), (z_target_view, wgpu::TextureFormat::Rgba8UnormSrgb));
    registry.textures.insert(TextureHandle(2), (depth_view, wgpu::TextureFormat::Depth32Float));
    registry.textures.insert(TextureHandle(3), (alpha_target_view, wgpu::TextureFormat::Rgba8UnormSrgb));

    let shader_src = "
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
        };

        @vertex
        fn vs_main(
            @builtin(vertex_index) in_vertex_index: u32,
            @builtin(instance_index) in_instance_index: u32,
        ) -> VertexOutput {
            var out: VertexOutput;
            let x = f32(i32(in_vertex_index) - 1) * 0.5;
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
            
            let offset_x = f32(in_instance_index) * 0.2 - 0.2;
            let offset_y = f32(in_instance_index) * 0.2 - 0.2;
            let z = 0.4 + f32(in_instance_index) * 0.2;

            out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
            
            if (in_instance_index == 0u) {
                out.color = vec4<f32>(1.0, 0.0, 0.0, 0.5);
            } else if (in_instance_index == 1u) {
                out.color = vec4<f32>(0.0, 1.0, 0.0, 0.5);
            } else {
                out.color = vec4<f32>(0.0, 0.0, 1.0, 0.5);
            }
            return out;
        }

        @fragment
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
            return in.color;
        }
    ";

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("VisualShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        immediate_size: 0,
    });

    let create_pipe = |depth: bool, blend: Option<wgpu::BlendState>| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                    blend,
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

    registry.pipelines.insert(PipelineHandle(1), create_pipe(true, Some(wgpu::BlendState::REPLACE)));
    registry.pipelines.insert(PipelineHandle(2), create_pipe(false, Some(wgpu::BlendState::ALPHA_BLENDING)));

    let mut pool = ifol_gpu::render::RenderNodePool::new();

    // --- Z-Buffer Test ---
    let mut graph_z = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width,
        height,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0])
    .with_depth_stencil(TextureHandle(2));

    graph_z.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..3,
        },
    )]);

    // --- Alpha Test ---
    let mut graph_alpha = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(3),
        width,
        height,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    graph_alpha.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(2),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..3,
        },
    )]);

    // --- 10K Objects Test ---
    let shader_10k_src = "
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
        };

        @vertex
        fn vs_main(
            @builtin(vertex_index) in_vertex_index: u32,
            @builtin(instance_index) in_instance_index: u32,
        ) -> VertexOutput {
            var out: VertexOutput;
            let x = f32(i32(in_vertex_index) - 1) * 0.015;
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.015;
            
            let row = f32(in_instance_index / 100u);
            let col = f32(in_instance_index % 100u);
            let offset_x = (col / 100.0) * 2.0 - 1.0 + 0.01;
            let offset_y = (row / 100.0) * 2.0 - 1.0 + 0.01;
            let z = 0.5;

            out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
            out.color = vec4<f32>(col / 100.0, row / 100.0, 1.0 - (col+row)/200.0, 1.0);
            return out;
        }

        @fragment
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
            return in.color;
        }
    ";
    let shader_10k = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("VisualShader10k"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_10k_src)),
    });

    let pipe_10k = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader_10k,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_10k,
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
    registry.pipelines.insert(PipelineHandle(3), pipe_10k);

    let (target_10k_view, target_10k_tex) = create_target();
    registry.textures.insert(TextureHandle(4), (target_10k_view, wgpu::TextureFormat::Rgba8UnormSrgb));

    let mut graph_10k = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(4),
        width,
        height,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    graph_10k.add_batch(&mut pool, vec![DrawCommand::new(
        PipelineHandle(3),
        DrawAction::Procedural {
            vertex_count: 3,
            instance_range: 0..10000,
        },
    )]);

    // --- State Tracking Accuracy Test ---
    let (target_interleaved_view, target_interleaved_tex) = create_target();
    registry.textures.insert(TextureHandle(5), (target_interleaved_view, wgpu::TextureFormat::Rgba8UnormSrgb));

    let mut graph_interleaved = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(5),
        width,
        height,
    })
    .with_clear_color([0.0, 0.0, 0.0, 1.0]);

    let interleaved_cmds = vec![
        DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ),
        DrawCommand::new(
            PipelineHandle(3),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ),
        DrawCommand::new(
            PipelineHandle(2),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ),
        DrawCommand::new(
            PipelineHandle(3),
            DrawAction::Procedural {
                vertex_count: 3,
                instance_range: 0..1,
            },
        ),
    ];
    graph_interleaved.add_batch(&mut pool, interleaved_cmds);

    // Execute
    let _ = executor.execute(&engine, &registry, &mut pool, &graph_z);
    let _ = executor.execute(&engine, &registry, &mut pool, &graph_alpha);
    let _ = executor.execute(&engine, &registry, &mut pool, &graph_10k);
    let idx_last = executor.execute(&engine, &registry, &mut pool, &graph_interleaved);


    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx_last),
        timeout: None,
    });

    fn save_texture(engine: &ifol_gpu::api::GpuEngine, texture: &wgpu::Texture, filename: &str) {
        let path = std::path::Path::new("examples/outputs").join(filename);
        engine.save_texture_to_file(texture, &path).expect("Lỗi lưu ảnh");
    }

    std::fs::create_dir_all("examples/outputs").unwrap();
    save_texture(&engine, &z_target_tex, "z_buffer_test.png");
    save_texture(&engine, &alpha_target_tex, "alpha_test.png");
    save_texture(&engine, &target_10k_tex, "10k_test.png");
    save_texture(&engine, &target_interleaved_tex, "interleaved_test.png");
}
