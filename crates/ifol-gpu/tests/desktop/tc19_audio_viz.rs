mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AudioUniform {
    freqs: [[f32; 4]; 4],
    base_color: [f32; 4],
    time: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

#[test]
fn run_tc19_audio_viz() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let tex_noise = h.load_texture("noise_perlin.jpeg");

        // The pipeline only uses group(0) texture and group(1) uniform
        let pipe_audio = h.register_pipeline(
            "audio_spectrum.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );

        // Simulate 16 frequency bands
        let simulated_freqs: [f32; 16] = [
            0.8, 0.9, 0.7, 0.5, // Bass
            0.4, 0.6, 0.8, 0.7, // Low Mids
            0.5, 0.4, 0.3, 0.5, // High Mids
            0.6, 0.7, 0.5, 0.3, // Treble
        ];

        let audio_uni = AudioUniform {
            freqs: [
                [simulated_freqs[0], simulated_freqs[1], simulated_freqs[2], simulated_freqs[3]],
                [simulated_freqs[4], simulated_freqs[5], simulated_freqs[6], simulated_freqs[7]],
                [simulated_freqs[8], simulated_freqs[9], simulated_freqs[10], simulated_freqs[11]],
                [simulated_freqs[12], simulated_freqs[13], simulated_freqs[14], simulated_freqs[15]],
            ],
            base_color: [0.0, 0.8, 1.0, 1.0], // Cyan Base
            time: 1.5,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };

        let bg_audio = h.create_custom_uniform_bind_group(audio_uni, "Audio Uniform");

        let (target_id, target_tex) = h.create_target("TC19 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_audio, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_audio, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC19 - Audio-Reactive Spectrum Visualizer",
            "features": [
                "Uniform Array Passing (vec4 packed frequencies)",
                "Procedural Neon Glow Shader",
                "Grid Background Generation",
                "Peak detection visualization"
            ],
            "bands": 16
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc19_audio_viz.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc19_audio_viz",
            "Audio-Reactive Spectrum Visualizer",
            "Trình diễn khả năng Graphic phản ứng theo âm thanh (Audio-Reactive). Shader nhận mảng tần số âm thanh qua Uniform Buffer và dựng ra thanh quang phổ Neon có độ phát sáng (Glow) và rớt điểm đỉnh (Peak detection) trên nền lưới Grid viễn tưởng.",
            "Xác thực khả năng truyền nhận mảng Uniform (Array Uniforms) và các thuật toán toán học tạo hình Neon.",
        );
    });
}
