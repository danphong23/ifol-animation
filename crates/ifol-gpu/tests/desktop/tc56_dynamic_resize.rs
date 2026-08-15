mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc56_dynamic_resize() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes_tex = h.load_texture("sprites_heroes.jpeg");
        let bg_tex = h.load_texture("bg_anime_city.jpg");

        let sprite_pipe = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let blit_pipe = h.register_pipeline(
            "sprite_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 1. Target 1 (400x600 - Portrait Left: Wizard)
        let (target_left, _tex_left) = h.create_custom_target(400, 600, "TC56 Target Left");
        let wizard_u = h.build_sprite_uniform(&heroes_tex, [0.0, -0.1], 0.85, [0.30, 0.0], [0.52, 1.0], 0.45, 0.12, 0.5, 1.0);
        let ubg_wizard = h.create_sprite_uniform_bind_group(wizard_u);
        let mut g_left = RenderGraph::new(RenderTarget::Offscreen { color: target_left, width: 400, height: 600 })
            .with_clear_color([0.1, 0.08, 0.15, 1.0]);
        g_left.add_batch(&mut h.pool, vec![
            DrawCommand::new(sprite_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, heroes_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_wizard, Vec::new())
        ]);
        h.executor.execute(&h.engine, &h.registry, &mut h.pool, &g_left).unwrap();

        // 2. Target 2 (400x600 - Portrait Right: Paladin)
        let (target_right, _tex_right) = h.create_custom_target(400, 600, "TC56 Target Right");
        let paladin_u = h.build_sprite_uniform(&heroes_tex, [0.0, -0.1], 0.85, [0.0, 0.0], [0.28, 1.0], 0.45, 0.12, 0.5, 1.0);
        let ubg_paladin = h.create_sprite_uniform_bind_group(paladin_u);
        let mut g_right = RenderGraph::new(RenderTarget::Offscreen { color: target_right, width: 400, height: 600 })
            .with_clear_color([0.08, 0.12, 0.15, 1.0]);
        g_right.add_batch(&mut h.pool, vec![
            DrawCommand::new(sprite_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, heroes_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_paladin, Vec::new())
        ]);
        h.executor.execute(&h.engine, &h.registry, &mut h.pool, &g_right).unwrap();

        // 3. Final Target (800x600) - Composite both resized viewports side by side onto background
        let (final_target, final_tex) = h.create_target("TC56 Final Composite");
        let bg_u = h.build_sprite_uniform(&bg_tex, [0.0, 0.0], 1.0, [0.0, 0.0], [1.0, 1.0], 0.0, 0.0, 0.9, 0.35);
        let ubg_bg = h.create_sprite_uniform_bind_group(bg_u);

        let bg_left_target = h.create_texture_bind_group(target_left, "Left Target BG");
        let bg_right_target = h.create_texture_bind_group(target_right, "Right Target BG");

        let left_quad_u = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [-0.5, 0.0],
            scale: [0.46, 0.92],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Left Quad Uniform");

        let right_quad_u = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.5, 0.0],
            scale: [0.46, 0.92],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Right Quad Uniform");

        let mut g_final = RenderGraph::new(RenderTarget::Offscreen { color: final_target, width: 800, height: 600 })
            .with_clear_color([0.04, 0.04, 0.06, 1.0]);
        g_final.add_batch(&mut h.pool, vec![
            // Background
            DrawCommand::new(sprite_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_bg, Vec::new()),
            // Left Panel (400x600 target)
            DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_left_target, Vec::new())
                .with_bind_group(1, left_quad_u, Vec::new()),
            // Right Panel (400x600 target)
            DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_right_target, Vec::new())
                .with_bind_group(1, right_quad_u, Vec::new()),
        ]);

        // Standard execute and record
        h.execute_and_record(
            &g_final,
            &final_tex,
            "tc56_dynamic_resize",
            "Dynamic Target Resizing & Viewport Composition",
            "Render Graph có thể cấp phát và kết xuất mượt mà qua các kích thước RenderTarget động (400x600 dọc và 800x600 ngang), sau đó tổng hợp thành công bố cục đa màn hình.",
            "Hai khung nhìn dọc 400x600 (Wizard bên trái, Paladin bên phải) hiển thị sắc nét, tỷ lệ chuẩn, hòa trộn hoàn hảo trên nền anime city 800x600."
        );

        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc56_dynamic_resize.json", serde_json::json!({
            "test_case": "TC56 - Dynamic Target Resizing"
        }).to_string()).unwrap();
    });
}
