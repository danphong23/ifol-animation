use std::f32::consts::PI;
use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[test]
fn test_tc75_fft() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc75_target");

        // Generate synthetic audio waveform (256 samples)
        // Contains 2 main frequencies (low and high)
        let sample_rate = 44100.0;
        let freq1 = 440.0;  // A4
        let freq2 = 5000.0; // High pitch
        
        let mut waveform = vec![0.0f32; 256];
        for i in 0..256 {
            let t = i as f32 / sample_rate;
            waveform[i] = (2.0 * PI * freq1 * t).sin() * 0.6 + (2.0 * PI * freq2 * t).sin() * 0.4;
        }

        let (_wave_buf_h, wave_buf) = h.create_storage_buffer(&waveform, "waveform_buf", wgpu::BufferUsages::empty());
        let spec_data = vec![0.0f32; 256];
        let (_spec_buf_h, spec_buf) = h.create_storage_buffer(&spec_data, "spectrum_buf", wgpu::BufferUsages::VERTEX);

        // Compute Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_fft_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        let compute_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_fft_bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &wave_buf, offset: 0, size: None }) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &spec_buf, offset: 0, size: None }) },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_fft.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_fft.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_shader_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_fft_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let p_fft = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_main"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let p_fft_h = h.insert_compute_pipeline(p_fft, vec![Some(1)]);

        // Render Pipeline
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_fft_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_fft_bg"),
            layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &spec_buf, offset: 0, size: None }) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_fft.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_fft.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_shader_code)),
        });

        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_fft_layout"), bind_group_layouts: &[Some(&render_bgl)], immediate_size: 0,
        });

        let render_pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_fft"), layout: Some(&render_layout),
            vertex: wgpu::VertexState { module: &render_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipe, vec![Some(2)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(p_fft_h, [1, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
        ]);

        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: 6, instance_range: 0..128 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![
            draw_cmd
        ]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("FFT Compute+Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc75_fft", "GPU Audio FFT", "Compute Shader 256-point FFT visualized with instances", "Render output");
    });
}
