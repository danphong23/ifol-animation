mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MaskingUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    key_color: [f32; 3],
    tolerance: f32,
    smoothness: f32,
    opacity: f32,
    _pad1: f32,
    _pad2: f32,
}

#[test]
fn run_tc21_masking() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_masking = h.register_pipeline(
            "masking.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // To make a perfect circle mask, the quad must be square on screen.
        // Screen is 800x600.
        // Let's make height 480px -> s_y = 480 / 600 = 0.8
        // Let's make width 480px -> s_x = 480 / 800 = 0.6
        let s_y = 0.8f32;
        let s_x = 0.6f32;

        let uniform = MaskingUniform {
            transform: [
                [s_x, 0.0, 0.0, 0.0],
                [0.0, s_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uv_min: [0.05, 0.05],
            uv_max: [0.25, 0.25], // Square crop
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.1,
            opacity: 1.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };

        let bg_sprite = h.create_custom_uniform_bind_group(uniform, "Masking Uniform");

        let (target_id, target_tex) = h.create_target("TC21 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_masking, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_sprite, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC21 - SDF Masking & Chroma Key",
            "features": [
                "Procedural SDF Masking",
                "Avatar Portrait clipping",
                "Texture UV projection",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc21_masking.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc21_masking",
            "SDF Masking (Avatar Portrait)",
            "Render nhân vật kết hợp thuật toán tách nền Chroma Key và cắt khung Procedural SDF hình tròn.",
            "Test khả năng cắt mask tuỳ biến (Avatar) giữ nguyên Aspect Ratio.",
        );
    });
}
