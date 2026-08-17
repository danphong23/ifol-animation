mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

#[test]
fn test_tc66_histogram() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load source image texture
        let heroes = h.load_texture("sprites_heroes.jpeg");
        let in_w = 800;
        let in_h = 600;

        // 2. Create Global Storage Buffer (256 u32 atomic counters initialized to 0)
        let initial_hist = vec![0u32; 256];
        let (_hist_buf_h, hist_buf) = h.create_storage_buffer(&initial_hist, "Histogram Storage Buffer", wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);

        // 3. Create Compute Bind Group Layout
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("histogram_compute_bg_layout"),
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = {
            let src_view = &h.registry.texture(&heroes.handle).unwrap().0;
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("histogram_compute_bind_group"),
                layout: &compute_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: hist_buf.as_entire_binding() },
                ],
            })
        };
        let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

        // 4. Register Compute Pipeline
        let compute_pipe_h = h.register_compute_pipeline("compute_histogram.wgsl", &[&compute_bg_layout]);

        // 5. Create Render Pass for Overlay Visualization
        let render_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("histogram_render_bg_layout"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let render_bind_group = {
            let src_view = &h.registry.texture(&heroes.handle).unwrap().0;
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("histogram_render_bind_group"),
                layout: &render_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&h.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: hist_buf.as_entire_binding() },
                ],
            })
        };
        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_histogram.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_histogram.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("histogram_render_pipe_layout"),
            bind_group_layouts: &[Some(&render_bg_layout)],
            immediate_size: 0,
        });

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("histogram_render_pipeline"),
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
        let render_bg_h = h.insert_bind_group(render_bind_group, 1);
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![Some(1)]);

        // 6. Build Graph
        let (target_handle, target_tex) = h.create_target("Histogram Visualizer Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        let workgroups_x = (in_w + 15) / 16;
        let workgroups_y = (in_h + 15) / 16;

        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(compute_pipe_h, [workgroups_x, workgroups_y, 1])
                .with_bind_group(0, compute_bg_h, Vec::new()),
        ]);

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

        // Warm Run
        // Need to clear buffer to 0 before dispatch again because graph execution appends
        h.engine.queue().write_buffer(&hist_buf, 0, bytemuck::cast_slice(&vec![0u32; 256]));
        
        let start_warm = Instant::now();
        let sub2 = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute warm failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub2),
            timeout: None,
        });
        let warm_time = start_warm.elapsed();

        // 7. Verify Data
        let hist_result: Vec<u32> = h.readback_storage_buffer(&hist_buf, 256);
        let total_pixels_processed: u32 = hist_result.iter().sum();
        let expected_pixels = in_w * in_h;
        println!("Histogram sum: {} (Expected: {}), Time: {:?}", total_pixels_processed, expected_pixels, warm_time);
        assert_eq!(total_pixels_processed, expected_pixels, "Histogram atomic sum does not match total pixel count!");

        // 8. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc66_histogram.png");
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).expect("Save failed");
    });
}
