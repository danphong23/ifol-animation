mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CRTUniform {
    curvature: [f32; 2],
    scanline_intensity: f32,
    time: f32,
}

#[test]
fn run_tc29_crt_vhs() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_scifi = h.load_texture("bg_scifi.jpeg");

        let pipe_crt = h.register_pipeline(
            "crt_vhs.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );

        let uniform = CRTUniform {
            curvature: [4.0, 4.0], // Barrel distortion curve factor
            scanline_intensity: 0.1,
            time: 1.5,
        };

        let bg_crt = h.create_custom_uniform_bind_group(uniform, "CRT Uniform");

        let (target_id, target_tex) = h.create_target("TC29 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_crt, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group, Vec::new())
                    .with_bind_group(1, bg_crt, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC29 - CRT & VHS Filter",
            "features": [
                "Barrel Distortion (Lens Curvature)",
                "Scanlines & Vignette",
                "RGB Split",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc29_crt_vhs.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc29_crt_vhs",
            "CRT & VHS Monitor Filter",
            "Hiệu ứng màn hình cong CRT cũ kỹ, kết hợp Scanlines (đường quét ngang), Vignette (tối góc) và Chromatic Aberration.",
            "Kiểm thử khả năng làm cong khung hình (Lens Distortion) kết hợp nhiều filter Post-Processing.",
        );
    });
}
