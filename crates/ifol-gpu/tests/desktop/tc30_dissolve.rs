mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DissolveUniform {
    threshold: f32,
    edge_width: f32,
    _pad0: [f32; 2],
    edge_color: [f32; 3],
    _pad1: f32,
}

#[test]
fn run_tc30_dissolve() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_noise = h.load_texture("noise_perlin.jpeg");

        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_dissolve = h.register_dual_texture_pipeline(
            "dissolve.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );

        let p_scale_y = 0.80f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin");

        let uniform = DissolveUniform {
            threshold: 0.5, // 50% dissolved
            edge_width: 0.05,
            _pad0: [0.0, 0.0],
            edge_color: [1.0, 0.4, 0.1], // Orange/Red burning edge
            _pad1: 0.0,
        };
        let bg_uniform = h.create_custom_uniform_bind_group(uniform, "Dissolve Uniform");
        
        let (target_a_id, _target_a_tex) = h.create_target("Target A");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_dual_tex = h.create_dual_texture_bind_group(target_a_id, tex_noise.handle, "Dissolve Dual Texture");

        // Pass 1: Extract Paladin via Chroma Key to transparent offscreen target
        let mut graph_chroma = RenderGraph::new(RenderTarget::Offscreen {
            color: target_a_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 0.0]);

        graph_chroma.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
            ],
        );

        // Pass 2: Dissolve the Paladin over dark gray background
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_dissolve, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_dual_tex, Vec::new())
                    .with_bind_group(1, bg_uniform, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC30 - Dissolve/Burn Transition",
            "features": [
                "2-Pass Rendering: Chroma Key -> Dissolve",
                "Noise-based threshold discard",
                "Glowing Edge computation",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc30_dissolve.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc30_dissolve",
            "Dissolve / Burn Transition",
            "Hiệu ứng tan biến hoặc cháy giấy. Sử dụng lệnh discard với Noise Map làm bản đồ độ cao (Height Map).",
            "Test lệnh discard và kỹ thuật viền sáng (Edge Glow) khi cắt alpha mask.",
        );
    });
}
