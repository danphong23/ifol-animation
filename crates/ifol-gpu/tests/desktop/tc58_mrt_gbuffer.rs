mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc58_mrt_gbuffer() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let tex_info = h.load_texture("sprites_heroes.jpeg");

        // 1. MRT Pipeline (2 Color Targets in single pass)
        let shader_path = std::path::Path::new("tests/shared_assets/shaders/mrt_gbuffer.wgsl");
        let shader_code = std::fs::read_to_string(shader_path).unwrap();
        let shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mrt_gbuffer"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });

        let layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mrt_layout"),
            bind_group_layouts: &[Some(&h.texture_bg_layout)],
            immediate_size: 0,
        });

        let mrt_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mrt_pipeline"),
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
                targets: &[
                    // Target 0: Albedo
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    // Target 1: Emissive
                    Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        // 2. Create the 2 Attachments in registry
        let (albedo_handle, _) = h.create_target("Albedo Target");
        let (emissive_handle, _) = h.create_target("Emissive Target");

        // Obtain views directly from the registry's owned texture so that downstream graph sampling accesses the rendered data
        let albedo_tex = h.registry.owned_texture(&albedo_handle).unwrap().clone();
        let emissive_tex = h.registry.owned_texture(&emissive_handle).unwrap().clone();

        let albedo_view = albedo_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let emissive_view = emissive_tex.create_view(&wgpu::TextureViewDescriptor::default());

        // Execute MRT Pass
        let mut encoder = h.engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("MRT Encoder") });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MRT Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &albedo_view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &emissive_view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rpass.set_pipeline(&mrt_pipeline);
            let bind_group = h.registry.bind_group(&tex_info.bind_group).unwrap();
            rpass.set_bind_group(0, bind_group, &[]);
            rpass.draw(0..6, 0..1);
        }
        let sub = h.engine.queue().submit(Some(encoder.finish()));
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });

        // Save individual attachment outputs
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        h.engine.save_texture_to_file_checked(&albedo_tex, std::path::Path::new("tests/outputs/desktop/tc58_mrt_albedo.png")).unwrap();
        h.engine.save_texture_to_file_checked(&emissive_tex, std::path::Path::new("tests/outputs/desktop/tc58_mrt_emissive.png")).unwrap();

        // 3. Side-by-Side Composite on Final 800x600 Canvas using sprite_blit
        let blit_pipe = h.register_pipeline(
            "sprite_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let (final_handle, final_tex) = h.create_target("TC58 Final Composite");

        let bg_albedo = h.create_texture_bind_group(albedo_handle, "Albedo BG");
        let bg_emissive = h.create_texture_bind_group(emissive_handle, "Emissive BG");

        let u_left = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [-0.5, 0.0],
            scale: [0.48, 0.95],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Albedo View Uniform");

        let u_right = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.5, 0.0],
            scale: [0.48, 0.95],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Emissive View Uniform");

        let mut g_final = RenderGraph::new(RenderTarget::Offscreen { color: final_handle, width: 800, height: 600 })
            .with_clear_color([0.05, 0.05, 0.08, 1.0]);

        g_final.add_batch(&mut h.pool, vec![
            // Left: Albedo Target
            DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_albedo, Vec::new())
                .with_bind_group(1, u_left, Vec::new()),
            // Right: Emissive Target
            DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_emissive, Vec::new())
                .with_bind_group(1, u_right, Vec::new()),
        ]);

        // Execute and record standard report
        h.execute_and_record(
            &g_final,
            &final_tex,
            "tc58_mrt_gbuffer",
            "Multiple Render Targets (MRT G-Buffer)",
            "Fragment shader xuất đồng thời 2 Attachments (Albedo và Emissive Mask) trong duy nhất 1 Render Pass (GBuffer). Bố cục xuất ảnh so sánh trực tiếp Albedo bên trái và Emissive Mask tách quang bên phải.",
            "Hai mục tiêu đệm màu (Color Targets) được điền đầy đủ và đồng bộ hoàn hảo trong 1 pass duy nhất; bên trái là hình ảnh gốc, bên phải là lớp mặt nạ phát sáng (emissive) trích xuất chính xác dải sáng rực."
        );

        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc58_mrt_gbuffer.json", serde_json::json!({
            "test_case": "TC58 - Multiple Render Targets"
        }).to_string()).unwrap();
    });
}
