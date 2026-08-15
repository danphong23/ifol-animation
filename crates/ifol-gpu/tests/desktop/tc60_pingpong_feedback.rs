mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc60_ping_pong() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let heroes_tex = h.load_texture("sprites_heroes.jpeg");

        // 1. Initial Sprite Pipeline
        let sprite_pipe = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 2. Ping-Pong Blit Pipeline
        let blit_pipe = h.register_pipeline(
            "ping_pong_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 3. Targets: Ping, Pong, Final
        let (ping_id, _ping_tex) = h.create_target("Ping");
        let (pong_id, _pong_tex) = h.create_target("Pong");
        let (final_id, final_tex) = h.create_target("Final Target");

        let bg_ping = h.create_texture_bind_group(ping_id, "Ping BG");
        let bg_pong = h.create_texture_bind_group(pong_id, "Pong BG");

        // Initial Wizard sprite in center of Ping target
        let wizard_u = h.build_sprite_uniform(&heroes_tex, [0.0, 0.0], 0.65, [0.30, 0.0], [0.52, 1.0], 0.45, 0.12, 0.5, 1.0);
        let ubg_wizard = h.create_sprite_uniform_bind_group(wizard_u);

        let mut g_init = RenderGraph::new(RenderTarget::Offscreen { color: ping_id, width: 800, height: 600 })
            .with_clear_color([0.05, 0.05, 0.08, 1.0]);
        g_init.add_batch(&mut h.pool, vec![
            DrawCommand::new(sprite_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, heroes_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_wizard, Vec::new())
        ]);
        h.executor.execute(&h.engine, &h.registry, &mut h.pool, &g_init).unwrap();

        // Feedback transform uniforms
        let u_zoom_out = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.008, 0.008],
            scale: [1.025, 1.025],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 0.85,
            _pad: 0.0,
        }, "Feedback Step 1 Uniform");

        let u_zoom_in = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [-0.005, -0.005],
            scale: [1.025, 1.025],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 0.85,
            _pad: 0.0,
        }, "Feedback Step 2 Uniform");

        // 4. Ping-Pong Multi-Pass Loop (Accumulating feedback trails without clearing)
        for _ in 0..8 {
            // Ping -> Pong
            let mut g_pong = RenderGraph::new(RenderTarget::Offscreen { color: pong_id, width: 800, height: 600 });
            g_pong.add_batch(&mut h.pool, vec![
                DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_ping, Vec::new())
                    .with_bind_group(1, u_zoom_out, Vec::new())
            ]);
            h.executor.execute(&h.engine, &h.registry, &mut h.pool, &g_pong).unwrap();

            // Pong -> Ping
            let mut g_ping = RenderGraph::new(RenderTarget::Offscreen { color: ping_id, width: 800, height: 600 });
            g_ping.add_batch(&mut h.pool, vec![
                DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_pong, Vec::new())
                    .with_bind_group(1, u_zoom_in, Vec::new())
            ]);
            h.executor.execute(&h.engine, &h.registry, &mut h.pool, &g_ping).unwrap();
        }

        // 5. Final Output Blit (1:1 Copy to Final Target)
        let u_copy = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.0, 0.0],
            scale: [1.0, 1.0],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Final Copy Uniform");

        let mut g_final = RenderGraph::new(RenderTarget::Offscreen { color: final_id, width: 800, height: 600 })
            .with_clear_color([0.02, 0.02, 0.04, 1.0]);
        g_final.add_batch(&mut h.pool, vec![
            DrawCommand::new(blit_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_ping, Vec::new())
                .with_bind_group(1, u_copy, Vec::new())
        ]);

        // Execute and record standard report
        h.execute_and_record(
            &g_final,
            &final_tex,
            "tc60_ping_pong",
            "Multi-Pass Ping-Pong Feedback Loop (Echo Trails)",
            "Chạy 16 passes RenderGraph luân phiên Ping -> Pong -> Ping mà không xoá buffer trung gian (LoadOp::Load), liên tục nhân bản tỷ lệ và làm mờ dần để tạo vệt đuôi chuyển động quang học (Optical Motion Echo Trails).",
            "Nhân vật Wizard ở trung tâm tạo ra chuỗi bóng mờ đồng tâm mở rộng dần với độ mờ đục giảm đều đặn, màu sắc mượt mà không bị vỡ kênh alpha hay nhiễu răng cưa."
        );

        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc60_ping_pong.json", serde_json::json!({
            "test_case": "TC60 - Ping Pong Feedback"
        }).to_string()).unwrap();
    });
}
