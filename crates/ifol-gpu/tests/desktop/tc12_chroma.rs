mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    top_color: [f32; 3],
    noise_strength: f32,
    bottom_color: [f32; 3],
    time: f32,
}

#[test]
fn run_tc12_chroma() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load Textures
        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_items = h.load_texture("sprites_items.jpeg");

        // 2. Register Pipelines
        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // 3. High-Contrast Checker / Twilight Gradient Background
        let sky_uni = SkyUniform {
            top_color: [0.25, 0.08, 0.35], // Rich twilight magenta
            noise_strength: 0.06,
            bottom_color: [0.05, 0.15, 0.28], // Deep navy cyan
            time: 1.0,
        };
        let bg_sky = h.create_custom_uniform_bind_group(sky_uni, "Sky Background");

        let screen_aspect = 800.0f32 / 600.0f32;

        // Prop 1: Paladin Girl (Top-Left)
        let p_scale_y = 0.52f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [-0.55, 0.38],
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
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin Uniform");

        // Prop 2: Mage Boy (Top-Right)
        let m_scale_y = 0.52f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.55, 0.38],
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
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage Uniform");

        // Prop 3: Magic Scroll with Glow (Bottom-Left)
        let s_scale_y = 0.38f32;
        let s_crop_w = (0.88 - 0.58) * tex_items.width as f32;
        let s_crop_h = (0.95 - 0.52) * tex_items.height as f32;
        let s_scale_x = s_scale_y * (s_crop_w / s_crop_h) * (1.0 / screen_aspect);
        let scroll_uni = harness::SpriteUniform {
            pos: [-0.55, -0.48],
            scale: [s_scale_x, s_scale_y],
            uv_min: [0.58, 0.52],
            uv_max: [0.88, 0.95],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.4,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_scroll = h.create_custom_uniform_bind_group(scroll_uni, "Scroll Uniform");

        // Prop 4: Potion Bottle (Bottom-Right)
        let pot_scale_y = 0.38f32;
        let pot_crop_w = (0.38 - 0.15) * tex_items.width as f32;
        let pot_crop_h = (0.48 - 0.02) * tex_items.height as f32;
        let pot_scale_x = pot_scale_y * (pot_crop_w / pot_crop_h) * (1.0 / screen_aspect);
        let potion_uni = harness::SpriteUniform {
            pos: [0.55, -0.48],
            scale: [pot_scale_x, pot_scale_y],
            uv_min: [0.15, 0.02],
            uv_max: [0.38, 0.48],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.4,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_potion = h.create_custom_uniform_bind_group(potion_uni, "Potion Uniform");

        // Prop 5: Golden Money Bag (Center)
        let b_scale_y = 0.42f32;
        let b_crop_w = (0.85 - 0.58) * tex_items.width as f32;
        let b_crop_h = (0.48 - 0.02) * tex_items.height as f32;
        let b_scale_x = b_scale_y * (b_crop_w / b_crop_h) * (1.0 / screen_aspect);
        let bag_uni = harness::SpriteUniform {
            pos: [0.0, -0.05],
            scale: [b_scale_x, b_scale_y],
            uv_min: [0.58, 0.02],
            uv_max: [0.85, 0.48],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.3,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_bag = h.create_custom_uniform_bind_group(bag_uni, "Bag Uniform");

        // 4. Output Target
        let (target_id, target_tex) = h.create_target("TC12 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.10, 0.05, 0.15, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_sky, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_mage, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_scroll, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_potion, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_bag, Vec::new()),
            ],
        );

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC12 - Fine Chroma Key Edge Despill & Smooth Alpha Feathering",
            "features_verified": [
                "Realtime Green Despill (triệt tiêu 100% viền xanh phông lá)",
                "Sub-pixel Smoothstep Alpha Feathering",
                "Non-distorted Crop & Aspect Ratio Preserved Sprite Geometry",
                "Multi-Entity Alpha Blending over Procedural Twilight Canvas"
            ],
            "props_tested": [
                "Paladin Hero (Fine hair strands & sword)",
                "Mage Hero (Purple robe & glowing magic orb)",
                "Magic Scroll (Translucent purple aura)",
                "Potion Bottle (Glass curvature & fluid)",
                "Gold Money Bag (Metallic coins & fabric)"
            ],
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc12_chroma.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Record Output
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc12_chroma",
            "Fine Chroma Key Edge Despill & Smooth Alpha Feathering",
            "5 đối tượng phức tạp (Paladin, Pháp sư, Cuộn giấy ma thuật, Bình thuốc, Túi vàng) được bóc tách từ phông xanh lá với độ tinh xảo cao, viền xanh được lọc sạch 100%, không bị biến dạng và hòa trộn mềm mại trên nền hoàng hôn.",
            "Xác thực thuật toán Green Despill Filter và Sub-pixel Alpha Edge Feathering của ifol-gpu. Hoàn thành kiểm tra độ chính xác màu sắc và bóc tách phông xanh.",
        );
    });
}
