mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlendUniform {
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[test]
fn run_tc53_blend_modes() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_blend = h.register_dual_texture_pipeline("blend_modes.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false);

        let blend_uni = BlendUniform {
            opacity: 1.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let bg_blend_uni = h.create_custom_uniform_bind_group(blend_uni, "Blend Uniform");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        // 8 Blend Modes Matrix (Base: Sci-Fi, Blend: Heroes Atlas)
        let dual_bg = h.create_dual_texture_bind_group(tex_scifi.handle, tex_heroes.handle, "SciFi + Heroes Blend");

        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_blend, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, dual_bg, Vec::new())
                    .with_bind_group(1, bg_blend_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC53 - Advanced 8 Blend Modes Matrix",
            "features": [
                "4x2 Split Screen Tile Comparison",
                "8 Industrial Blend Equations (Normal, Multiply, Screen, Overlay, HardLight, SoftLight, ColorDodge, Difference)",
                "Sub-tile Aspect-Ratio Corrected Hero Sampling"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc53_blend_modes.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc53_blend_modes",
            "Advanced 8 Blend Modes Matrix",
            "Ma trận so sánh 8 chế độ hòa trộn lớp chuẩn After Effects / Photoshop: Màn hình chia thành 8 ô (Normal, Multiply, Screen, Overlay, Hard Light, Soft Light, Color Dodge, Difference) giữa nhân vật Paladin và nền thành phố Sci-Fi với tỷ lệ khung hình tự nhiên không bị méo.",
            "Xác thực bảng công thức toán học hòa trộn màu sắc (Photoshop Blend Equations) trực tiếp trong Fragment Shader.",
        );
    });
}
