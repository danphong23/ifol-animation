mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RippleUniform {
    center: [f32; 2],
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
    _pad: [f32; 2],
}

#[test]
fn run_tc28_ripple() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_city = h.load_texture("bg_anime_city.jpg");

        let pipe_ripple = h.register_pipeline(
            "ripple.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );

        let uniform = RippleUniform {
            center: [0.5, 0.5], // Center of screen
            time: 5.0, // Arbitrary time
            amplitude: 0.02,
            frequency: 40.0,
            speed: 2.0,
            _pad: [0.0, 0.0],
        };

        let bg_ripple = h.create_custom_uniform_bind_group(uniform, "Ripple Uniform");

        let (target_id, target_tex) = h.create_target("TC28 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_ripple, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_city.bind_group, Vec::new())
                    .with_bind_group(1, bg_ripple, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC28 - Ripple (Water/Shockwave Distortion)",
            "features": [
                "Sin/Cos UV Displacement",
                "Distance-based damping",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc28_ripple.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc28_ripple",
            "Ripple (Water/Shockwave Distortion)",
            "Hiệu ứng lượn sóng nước hoặc sóng xung kích (Shockwave) lan tỏa từ một tâm điểm. UV bị bóp méo theo hàm Sin/Cos.",
            "Thử nghiệm bóp méo không gian 2D theo hướng tỏa tròn từ một tâm động.",
        );
    });
}
