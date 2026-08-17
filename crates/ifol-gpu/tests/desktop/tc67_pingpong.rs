mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

#[test]
fn test_tc67_pingpong() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let in_w = 800;
        let in_h = 600;

        // 1. Create two storage textures A and B for ping-pong
        let (_tex_a_h, tex_a) = h.create_storage_texture(in_w, in_h, wgpu::TextureFormat::Rgba8Unorm, "RD Texture A");
        let (_tex_b_h, tex_b) = h.create_storage_texture(in_w, in_h, wgpu::TextureFormat::Rgba8Unorm, "RD Texture B");

        let view_a = tex_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex_b.create_view(&wgpu::TextureViewDescriptor::default());

        // 2. Initialize Texture A with seed data
        let mut seed_data = vec![0u8; (in_w * in_h * 4) as usize];
        for y in 0..in_h {
            for x in 0..in_w {
                let idx = ((y * in_w + x) * 4) as usize;
                // Default: A=1, B=0
                seed_data[idx] = 255;
                seed_data[idx + 1] = 0;
                seed_data[idx + 2] = 0;
                seed_data[idx + 3] = 255;

                // Seed some squares
                if x > 380 && x < 420 && y > 280 && y < 320 {
                    seed_data[idx + 1] = 255; // B = 1
                }
                if x > 200 && x < 220 && y > 400 && y < 420 {
                    seed_data[idx + 1] = 255;
                }
                if x > 600 && x < 620 && y > 150 && y < 170 {
                    seed_data[idx + 1] = 255;
                }
            }
        }
        h.engine.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex_a,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &seed_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * in_w),
                rows_per_image: Some(in_h),
            },
            wgpu::Extent3d {
                width: in_w,
                height: in_h,
                depth_or_array_layers: 1,
            }
        );

        // 3. Create Compute Bind Group Layout
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rd_compute_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        // Bind Group A -> B (Read A, Write B)
        let compute_bind_group_a2b = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd_compute_bg_a2b"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_b) },
            ],
        });
        let bg_a2b_h = h.insert_bind_group(compute_bind_group_a2b, 1);

        // Bind Group B -> A (Read B, Write A)
        let compute_bind_group_b2a = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd_compute_bg_b2a"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_b) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_a) },
            ],
        });
        let bg_b2a_h = h.insert_bind_group(compute_bind_group_b2a, 1);

        let compute_pipe_h = h.register_compute_pipeline("compute_reaction_diffusion.wgsl", &[&compute_bg_layout]);

        // 4. Create Render Pass
        let render_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rd_render_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        });

        let render_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd_render_bind_group"),
            layout: &render_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&h.sampler) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bind_group, 1);

        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_reaction_diffusion.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_reaction_diffusion.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rd_render_pipe_layout"),
            bind_group_layouts: &[Some(&render_bg_layout)],
            immediate_size: 0,
        });

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rd_render_pipeline"),
            layout: Some(&render_pipe_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![Some(1)]);

        // 5. Build Graph
        let (target_handle, target_tex) = h.create_target("RD Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        let workgroups_x = (in_w + 15) / 16;
        let workgroups_y = (in_h + 15) / 16;

        // Perform 40 steps (20 ping-pongs) per frame for faster evolution
        let mut compute_commands = Vec::new();
        for _ in 0..20 {
            compute_commands.push(
                ComputeCommand::new(compute_pipe_h, [workgroups_x, workgroups_y, 1])
                    .with_bind_group(0, bg_a2b_h, Vec::new())
            );
            compute_commands.push(
                ComputeCommand::new(compute_pipe_h, [workgroups_x, workgroups_y, 1])
                    .with_bind_group(0, bg_b2a_h, Vec::new())
            );
        }
        graph.add_compute_batch(&mut pool, compute_commands);

        graph.add_batch(&mut pool, vec![
            DrawCommand::new(
                render_pipe_h,
                DrawAction::Procedural {
                    vertex_count: 3,
                    instance_range: 0..1,
                },
            ).with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        // Cold Run
        let start_cold = Instant::now();
        let sub1 = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub1),
            timeout: None,
        });
        let _cold_time = start_cold.elapsed();

        // Warm Run - execute again to evolve more
        let start_warm = Instant::now();
        let sub2 = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute warm failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub2),
            timeout: None,
        });
        let warm_time = start_warm.elapsed();

        // Evolve a few more times for a visible pattern (total ~2500 steps)
        for _ in 0..60 {
            let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute failed");
            let _ = h.engine.device().poll(wgpu::PollType::Wait {
                submission_index: Some(sub),
                timeout: None,
            });
        }

        println!("Reaction Diffusion Time (1 Frame / 40 steps): {:?}", warm_time);

        // 6. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc67_pingpong.png");
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).expect("Save failed");
    });
}
