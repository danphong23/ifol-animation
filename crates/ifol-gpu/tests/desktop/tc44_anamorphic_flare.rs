mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FlareUniform {
    threshold: f32,
    streak_length: f32,
    intensity: f32,
    _pad0: f32,
    tint_color: [f32; 4],
}

#[test]
fn run_tc44_anamorphic_flare() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_flare = h.register_pipeline("anamorphic_flare.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        let flare_uni = FlareUniform {
            threshold: 0.35,
            streak_length: 3.5,
            intensity: 2.2,
            _pad0: 0.0,
            tint_color: [0.15, 0.65, 1.0, 1.0], // Sci-Fi Hollywood anamorphic blue
        };
        let bg_flare_uni = h.create_custom_uniform_bind_group(flare_uni, "Flare Uniform");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_flare, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_flare_uni, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC44 - Anamorphic Lens Flare & Horizontal Streaks",
            "features": [
                "1D Horizontal Exponential Kernel",
                "Spectral Dispersion Color Tinting",
                "Edge Boundary Clamping & Falloff Smoothing"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc44_anamorphic_flare.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &final_target_tex,
            "tc44_anamorphic_flare",
            "Anamorphic Lens Flare",
            "Hiệu ứng tia sáng kéo dãn ngang (Anamorphic Streak) đặc trưng của ống kính điện ảnh. Ánh sáng từ các điểm chói lòa trong khung cảnh Sci-Fi được tích lũy theo trục X kèm quang sai màu xanh lam.",
            "Xác thực thuật toán lấy mẫu 1D bán kính rộng (33 taps) có xử lý khử viền đen (Boundary Falloff Clamping).",
        );
    });
}
