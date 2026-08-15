mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GodRaysUniform {
    light_pos: [f32; 2],
    exposure: f32,
    decay: f32,
    density: f32,
    weight: f32,
    _pad: [f32; 2],
}

#[test]
fn run_tc27_godrays() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_forest = h.load_texture("bg_forest.jpeg");

        let pipe_godrays = h.register_pipeline(
            "godrays.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let uniform = GodRaysUniform {
            light_pos: [0.5, 0.2], // Light source near top center
            exposure: 0.8,
            decay: 0.95,
            density: 0.8,
            weight: 0.05,
            _pad: [0.0, 0.0],
        };

        let bg_godrays = h.create_custom_uniform_bind_group(uniform, "GodRays Uniform");

        let (target_id, target_tex) = h.create_target("TC27 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_godrays, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_godrays, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC27 - GodRays (Volumetric Light Shafts)",
            "features": [
                "Radial Blur algorithm",
                "Heavy Texture Sampling loop in Fragment Shader",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc27_godrays.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc27_godrays",
            "GodRays (Volumetric Light Shafts)",
            "Hiệu ứng Tia Sáng sử dụng kỹ thuật Radial Blur (lấy mẫu mờ tỏa tròn từ tâm sáng).",
            "Đo năng lực tính toán vòng lặp lấy mẫu (heavy texture sampling loop) trong Fragment Shader.",
        );
    });
}
