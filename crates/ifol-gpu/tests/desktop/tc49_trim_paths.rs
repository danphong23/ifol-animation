mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TrimPathsUniform {
    center: [f32; 2],
    half_size: [f32; 2],
    corner_radius: f32,
    thickness: f32,
    dash_length: f32,
    gap_length: f32,
    dash_offset: f32,
    trim_start: f32,
    trim_end: f32,
    _pad0: f32,
    stroke_color: [f32; 4],
}

#[test]
fn run_tc49_trim_paths() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_trim = h.register_pipeline("trim_paths.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Mage in center
        let m_scale_y = 0.75f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [m_scale_x, m_scale_y],
            uv_min: [0.27, 0.01],
            uv_max: [0.54, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage");

        // Trim Paths animated neon bounding card around Mage
        let trim_uni = TrimPathsUniform {
            center: [0.5, 0.5],
            half_size: [0.18, 0.38],
            corner_radius: 0.04,
            thickness: 0.006,
            dash_length: 0.6,
            gap_length: 0.4,
            dash_offset: 2.5,
            trim_start: 0.05,
            trim_end: 0.90, // Trims 85% around the box
            _pad0: 0.0,
            stroke_color: [0.1, 0.9, 1.0, 1.0], // Neon Cyan Glowing Stroke
        };
        let bg_trim_uni = h.create_custom_uniform_bind_group(trim_uni, "TrimPaths Uniform");

        let (target_scene_id, _target_scene_tex) = h.create_target("Target Scene");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_scene = h.create_texture_bind_group(target_scene_id, "Scene Texture BG");

        // Pass 1: Render scene (Sci-Fi BG + Mage)
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
                    .with_bind_group(1, bg_mage.clone(), Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_mage.clone(), Vec::new()),
            ],
        );

        // Pass 2: Draw Animated Trim Paths & Dashed Glowing Vector Stroke around Mage
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_trim, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_scene, Vec::new())
                    .with_bind_group(1, bg_trim_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_scene).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC49 - Animated Trim Paths & Dashed Vector Stroke",
            "features": [
                "Parametric Arc Length Along SDF Boundary",
                "Procedural Dash and Gap Generation",
                "Trim Paths Start and End Truncation with Neon Glow"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc49_trim_paths.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc49_trim_paths",
            "Animated Trim Paths",
            "Tính năng Trim Paths vector của After Effects: Tạo khung viền bo tròn bọc quanh Pháp Sư với các đoạn nét đứt neon (Dashed Line) tự động tính toán theo chu vi và cắt ngắn theo phần trăm (Trim Start/End).",
            "Xác thực thuật toán tham số hóa chiều dài cung viền (Arc Length Parameterization) trên hàm khoảng cách SDF.",
        );
    });
}
