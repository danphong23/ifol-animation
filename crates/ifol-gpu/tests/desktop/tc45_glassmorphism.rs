mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlassUniform {
    panel_center: [f32; 2],
    panel_size: [f32; 2],
    corner_radius: f32,
    blur_amount: f32,
    refraction_strength: f32,
    border_thickness: f32,
}

#[test]
fn run_tc45_glassmorphism() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_glass = h.register_pipeline("glassmorphism.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Paladin on left
        let p_scale_y = 0.70f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [-0.35, 0.0],
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

        // Floating Frosted Glass UI card parameters
        let glass_uni = GlassUniform {
            panel_center: [0.55, 0.50], // Centered slightly on right side
            panel_size: [0.25, 0.28],   // 50% width, 56% height
            corner_radius: 0.035,       // Rounded corners
            blur_amount: 3.5,           // Frosted blur
            refraction_strength: 0.015, // Optical edge bending
            border_thickness: 0.005,    // Specular Fresnel rim
        };
        let bg_glass_uni = h.create_custom_uniform_bind_group(glass_uni, "Glass Uniform");

        let (target_scene_id, _target_scene_tex) = h.create_target("Target Scene");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_scene = h.create_texture_bind_group(target_scene_id, "Scene Texture BG");

        // Pass 1: Render rich scene (SciFi background + Paladin) into Target Scene
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
                    .with_bind_group(1, bg_paladin, Vec::new()),
            ],
        );

        // Pass 2: Apply Glassmorphism Panel on top of Target Scene
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_glass, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_scene, Vec::new())
                    .with_bind_group(1, bg_glass_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_scene).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC45 - Frosted Glassmorphism Panel",
            "features": [
                "Backdrop Fetch & Frosted Gaussian Blur",
                "SDF Rounded Box Gradient Refraction",
                "Directional Specular Rim Lighting (Fresnel Effect)"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc45_glassmorphism.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc45_glassmorphism",
            "Frosted Glassmorphism Panel",
            "Hiệu ứng giao diện kính mờ (Frosted Glass UI) cao cấp: Lấy mẫu khung cảnh nền phía sau (Backdrop), làm mờ mềm mại kết hợp khúc xạ viền kính (Refraction) và viền sáng phản xạ (Specular Fresnel Rim).",
            "Xác thực sự kết hợp hoàn hảo giữa toán học hình học Signed Distance Field (SDF) và kỹ thuật Post-Processing lọc nền.",
        );
    });
}
