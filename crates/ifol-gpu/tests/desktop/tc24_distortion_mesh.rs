mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use wgpu::util::DeviceExt;
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DistortionUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    time: f32,
    amplitude: f32,
    frequency: f32,
    _pad: f32,
}

#[test]
fn run_tc24_distortion_mesh() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_distortion = h.register_pipeline(
            "distortion.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let s_y = 1.5f32;
        let s_x = s_y * (600.0 / 800.0) * (0.275 / 0.97);

        let uniform = DistortionUniform {
            transform: [
                [s_x, 0.0, 0.0, 0.0],
                [0.0, s_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            time: 1.5, // Hardcoded time to show the sway in a still image
            amplitude: 0.3,
            frequency: 2.0,
            _pad: 0.0,
        };

        let bg_distortion = h.create_custom_uniform_bind_group(uniform, "Distortion Uniform");

        let (target_id, target_tex) = h.create_target("TC24 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_distortion, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_distortion, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC24 - Vertex Deformation",
            "features": [
                "Vertex shader offset based on Y-axis (Sway)",
                "Time-based evaluation (Wind simulation)",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc24_distortion_mesh.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc24_distortion_mesh",
            "Vertex Deformation (Wind/Sway)",
            "Mô phỏng hiệu ứng gió thổi (Wind/Sway) bằng cách tác động lên các đỉnh (vertices) của Sprite theo hàm sin(time). Phần dưới của sprite được neo (anchor) và phần trên bị uốn cong.",
            "Xác thực khả năng tạo motion động trên GPU mà không cần tạo xương (bone) hay frame-by-frame animation.",
        );
    });
}
