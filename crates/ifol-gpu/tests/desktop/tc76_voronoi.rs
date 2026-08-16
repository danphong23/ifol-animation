use std::time::Instant;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[test]
fn test_tc76_voronoi() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc76_target");

        let bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("voronoi_bgl"),
            entries: &[],
        });
        
        let bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("voronoi_bg"), layout: &bgl, entries: &[],
        });
        let bg_h = h.insert_bind_group(bg, 1);

        let shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_voronoi.wgsl").unwrap();
        let shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_voronoi.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });

        let layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("voronoi_layout"), bind_group_layouts: &[Some(&bgl)], immediate_size: 0,
        });

        let pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_voronoi"), layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let pipe_h = h.insert_pipeline(pipe, vec![Some(1)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        let draw_cmd = DrawCommand::new(
            pipe_h,
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        ).with_bind_group(0, bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Procedural Voronoi Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc76_voronoi", "Procedural Voronoi Noise", "Fullscreen triangle with Cellular Voronoi Noise generated in Fragment Shader", "Render output");
    });
}
