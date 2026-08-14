mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use wgpu::util::DeviceExt;
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlitchUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    time: f32,
    intensity: f32,
    aberration: f32,
    _pad: f32,
}

#[test]
fn run_tc26_glitch() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_glitch = h.register_pipeline(
            "glitch.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let s_y = 1.5f32;
        let s_x = s_y * (600.0 / 800.0) * (0.275 / 0.97);

        let uniform = GlitchUniform {
            transform: [
                [s_x, 0.0, 0.0, 0.0],
                [0.0, s_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            time: 2.34, // Arbitrary time to get a good glitch frame
            intensity: 0.8,
            aberration: 0.015, // RGB shift distance
            _pad: 0.0,
        };

        let bg_glitch = h.create_custom_uniform_bind_group(uniform, "Glitch Uniform");

        let (target_id, target_tex) = h.create_target("TC26 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_glitch, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_glitch, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC26 - Glitch & Chromatic Aberration",
            "features": [
                "Time-based UV horizontal slice shifting",
                "RGB channel separation (Chromatic Aberration)",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc26_glitch.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc26_glitch",
            "Glitch & Chromatic Aberration",
            "Sử dụng kỹ thuật dịch chuyển kênh màu (RGB Split/Chromatic Aberration) kết hợp với các dải nhiễu ngang (Horizontal Block Noise) theo biến thời gian (time).",
            "Mô phỏng hiệu ứng Glitch kiểu Cyberpunk/Retro hoặc hiệu ứng chuyển cảnh (Transition) mạnh mẽ trực tiếp trên Sprite 2D.",
        );
    });
}
