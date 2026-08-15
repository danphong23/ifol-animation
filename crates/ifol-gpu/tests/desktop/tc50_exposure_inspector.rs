mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ExposureUniform {
    zebra_threshold: f32,
    zebra_speed: f32,
    time: f32,
    mode: f32,
}

#[test]
fn run_tc50_exposure_inspector() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_inspector = h.register_pipeline("exposure_inspector.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Paladin in center
        let p_scale_y = 0.80f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin");

        let exp_uni = ExposureUniform {
            zebra_threshold: 0.80, // Highlights > 80% get zebra stripes
            zebra_speed: 2.0,
            time: 1.0,
            mode: 0.0, // Split screen mode (Left: Zebra, Right: ARRI False Color)
        };
        let bg_exp_uni = h.create_custom_uniform_bind_group(exp_uni, "Exposure Uniform");

        let (target_scene_id, _target_scene_tex) = h.create_target("Target Scene");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_scene = h.create_texture_bind_group(target_scene_id, "Scene Texture BG");

        // Pass 1: Render complete scene (Sci-Fi BG + Paladin)
        let mut graph_scene = RenderGraph::new(RenderTarget::Offscreen {
            color: target_scene_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_scene.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_screen, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin.clone(), Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin.clone(), Vec::new()),
            ],
        );

        // Pass 2: Apply Exposure Inspector (Left: Zebra Stripes, Right: False Color Spectrum)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_inspector, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_scene, Vec::new())
                    .with_bind_group(1, bg_exp_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_scene).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC50 - False Color & Zebra Exposure Inspector",
            "features": [
                "IRE Luminance Histogram Analysis",
                "Procedural Diagonal Zebra Stripes Animation",
                "ARRI / DaVinci Professional False Color Heatmap"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc50_exposure_inspector.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc50_exposure_inspector",
            "Exposure Inspector Overlay",
            "Công cụ kiểm tra phơi sáng chuẩn phòng thu điện ảnh (DaVinci/ARRI False Color): Nửa trái hiển thị vạch sọc ngựa vằn (Zebra Stripes) tại vùng cháy sáng, nửa phải hiển thị bản đồ nhiệt IRE (Tím = Tối, Hồng = Da người, Đỏ = Cháy sáng).",
            "Xác thực thuật toán phân tích mức IRE tức thời và tạo Overlay kỹ thuật trực tiếp trên GPU.",
        );
    });
}
