mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AspectFillUniform {
    target_aspect: f32,
    source_aspect: f32,
    blur_strength: f32,
    shadow_opacity: f32,
}

#[test]
fn run_tc41_aspect_fill() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        // Vertical 9:16 canvas simulated in 450x800 resolution
        let mut h = DesktopTestHarness::new(450, 800).await;
        
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_fill = h.register_pipeline("aspect_fill.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        let target_aspect = 450.0f32 / 800.0f32; // ~0.5625 (TikTok/Shorts format)
        let source_aspect = tex_scifi.width as f32 / tex_scifi.height as f32; // Landscape ~1.777

        let fill_uni = AspectFillUniform {
            target_aspect,
            source_aspect,
            blur_strength: 2.5,
            shadow_opacity: 0.6,
        };
        let bg_fill_uni = h.create_custom_uniform_bind_group(fill_uni, "AspectFill Uniform");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 450,
            height: 800,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_fill, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_fill_uni, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC41 - Auto Aspect Ratio Adaptation & Background Blur Fill",
            "features": [
                "Dynamic Aspect Ratio Transformation",
                "Non-Uniform Canvas Pillarbox Elimination",
                "Background Gaussian Blur Fill with Drop Shadow"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc41_aspect_fill.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &final_target_tex,
            "tc41_aspect_fill",
            "Aspect Ratio Blur Fill",
            "Tự động thích ứng ảnh ngang 16:9 vào khung dọc 9:16 (TikTok/Shorts). Phóng đại nền và làm mờ Gaussian để triệt tiêu dải đen, giữ nguyên tỷ lệ sắc nét cho khung trung tâm.",
            "Xác thực thuật toán chuyển đổi không gian UV động giữa Target Aspect Ratio và Source Aspect Ratio.",
        );
    });
}
