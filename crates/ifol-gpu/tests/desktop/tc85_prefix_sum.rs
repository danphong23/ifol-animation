use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[test]
fn test_tc85_prefix_sum() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc85_target");

        // Prepare 256 input integers
        let input_data: Vec<u32> = (0..256).map(|i| (i % 5) + 1).collect();
        let expected_first = 0;
        let expected_second = input_data[0];

        let (_, data_buf) = h.create_storage_buffer(&input_data, "prefix_data", wgpu::BufferUsages::empty());

        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("prefix_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bg_compute = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("prefix_compute_bg"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &data_buf, offset: 0, size: None }) },
            ],
        });
        let bg_compute_h = h.insert_bind_group(bg_compute, 1);

        let compute_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_prefix_sum.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_prefix_sum.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("prefix_compute_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let compute_pipe = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_prefix_sum"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let compute_pipe_h = h.insert_compute_pipeline(compute_pipe, vec![Some(1)]);

        // Render Pipeline
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("prefix_render_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("prefix_render_bg"), layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &data_buf, offset: 0, size: None }) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let render_code = std::fs::read_to_string("tests/shared_assets/shaders/render_prefix_sum.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_prefix_sum.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_code)),
        });

        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("prefix_render_layout"), bind_group_layouts: &[Some(&render_bgl)], immediate_size: 0,
        });

        let render_pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_prefix_sum"), layout: Some(&render_layout),
            vertex: wgpu::VertexState { module: &render_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipe, vec![Some(2)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [1, 1, 1]).with_bind_group(0, bg_compute_h, vec![]),
        ]);

        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: 256 * 6, instance_range: 0..1 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Prefix Sum Render Time: {:?}", t_elapsed);

        // Readback to verify exact correctness
        let result_data: Vec<u32> = h.readback_storage_buffer(&data_buf, 256);
        assert_eq!(result_data[0], expected_first);
        assert_eq!(result_data[1], expected_second);
        println!("Prefix Sum Readback Validated Successfully: [0]={}, [1]={}", result_data[0], result_data[1]);

        h.execute_and_record(&graph, &target_tex, "tc85_prefix_sum", "Compute Prefix Sum / Scan", "Parallel Exclusive Scan (Blelloch algorithm) on GPU with bar chart visualization", "Render output");
    });
}
