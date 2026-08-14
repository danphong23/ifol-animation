mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TransitionUniform {
    progress: f32,
    effect_type: f32,
    direction_x: f32,
    direction_y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    top_color: [f32; 3],
    noise_strength: f32,
    bottom_color: [f32; 3],
    time: f32,
}

#[test]
fn run_tc18_transition() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_sky = h.register_sky_pipeline();
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_transition = h.register_transition_pipeline();

        let screen_aspect = 800.0f32 / 600.0f32;

        // --- SCENE A (Purple Sky + Paladin) ---
        let sky_a_uni = SkyUniform {
            top_color: [0.15, 0.08, 0.25], // Purple
            noise_strength: 0.02,
            bottom_color: [0.4, 0.15, 0.3], // Magenta
            time: 1.0,
        };
        let bg_sky_a = h.create_custom_uniform_bind_group(sky_a_uni, "Sky A");

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
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin A");

        // --- SCENE B (Winter Blue Sky + Mage) ---
        let sky_b_uni = SkyUniform {
            top_color: [0.05, 0.1, 0.2], // Dark Blue
            noise_strength: 0.05,
            bottom_color: [0.2, 0.4, 0.6], // Light Blue
            time: 2.0,
        };
        let bg_sky_b = h.create_custom_uniform_bind_group(sky_b_uni, "Sky B");

        let m_scale_y = 0.80f32;
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
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage B");

        // --- TRANSITION CONFIG ---
        let trans_uni = TransitionUniform {
            progress: 0.5, // 50% Transition
            effect_type: 1.0, // 1.0 = Glitch, 0.0 = Liquid Warp
            direction_x: 1.0,
            direction_y: 0.0,
        };
        let bg_trans_uniform = h.create_custom_uniform_bind_group(trans_uni, "Transition Uniform");

        // Targets
        let (target_a_id, target_a_tex) = h.create_target("Target A");
        let (target_b_id, target_b_tex) = h.create_target("Target B");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_dual_tex = h.create_dual_texture_bind_group(target_a_id, target_b_id, "Dual Texture Transition");

        // Pass 1: Target A
        let mut graph_a = RenderGraph::new(RenderTarget::Offscreen { color: target_a_id, width: 800, height: 600 }).with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_a.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_noise.bind_group.clone(), Vec::new()).with_bind_group(1, bg_sky_a, Vec::new()),
            DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new()).with_bind_group(1, bg_paladin, Vec::new()),
        ]);

        // Pass 2: Target B
        let mut graph_b = RenderGraph::new(RenderTarget::Offscreen { color: target_b_id, width: 800, height: 600 }).with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_b.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_noise.bind_group.clone(), Vec::new()).with_bind_group(1, bg_sky_b, Vec::new()),
            DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new()).with_bind_group(1, bg_mage, Vec::new()),
        ]);

        // Pass 3: Transition (A -> B)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen { color: final_target_id, width: 800, height: 600 }).with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_final.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_transition, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_dual_tex, Vec::new())
                .with_bind_group(1, bg_trans_uniform, Vec::new()),
        ]);

        // Execute all passes
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_a).unwrap();
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_b).unwrap();
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).unwrap();

        let graph_json = serde_json::json!({
            "test_case": "TC18 - Video Transition Effects (Glitch)",
            "features": [
                "Dual-Texture Bind Groups",
                "Procedural Glitch Effect (RGB Split & Block Shift)",
                "3-Pass Render Graph execution"
            ],
            "passes": 3
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc18_transition.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc18_transition",
            "Video Transition Effects (Glitch / Liquid Warp)",
            "Chuyển cảnh (Transition) từ Cảnh A (Paladin) sang Cảnh B (Mage). Thuật toán đang dùng là 'Glitch' với độ gắt 50% (Progress = 0.5). Hình ảnh bị cắt xẻ (Block shift) và quang sai màu (RGB Split) theo thời gian thực.",
            "Xác thực khả năng đọc 2 luồng Texture song song (Dual-Texture Bind Group) để tạo hiệu ứng chuyển cảnh chuyên nghiệp trong quy trình Video Editing.",
        );
    });
}
