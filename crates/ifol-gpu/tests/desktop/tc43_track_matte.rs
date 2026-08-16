mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TrackMatteUniform {
    matte_type: f32, // 0 = Alpha Matte, 1 = Inverted Alpha, 2 = Luma, 3 = Inverted Luma
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
}

#[test]
fn run_tc43_track_matte() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_track_matte = h.register_dual_texture_pipeline("track_matte.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false);

        // Paladin used as Matte mask
        let p_scale_y = 0.85f32;
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

        let matte_uni = TrackMatteUniform {
            matte_type: 0.0, // Alpha Matte: Reveal Sci-Fi texture only inside Paladin silhouette!
            opacity: 1.0,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        let bg_matte_uni = h.create_custom_uniform_bind_group(matte_uni, "TrackMatte Uniform");

        let (target_matte_id, _target_matte_tex) = h.create_target("Target Matte");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        // Pass 1: Extract Paladin Alpha into Target Matte
        let mut graph_chroma = RenderGraph::new(RenderTarget::Offscreen {
            color: target_matte_id,
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

        // Pass 2: Track Matte (Base: Sci-Fi BG Texture, Matte: Target Matte Paladin)
        let dual_bg = h.create_dual_texture_bind_group(tex_scifi.handle, target_matte_id, "SciFi + Paladin Matte");

        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.08, 0.08, 0.12, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_track_matte, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, dual_bg, Vec::new())
                    .with_bind_group(1, bg_matte_uni, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC43 - Dual-Layer Track Matte (Luma & Alpha Masking)",
            "features": [
                "Dual-Texture BindGroup Pipeline",
                "Dynamic Alpha & Luma Track Masking",
                "Video Compositor Stencil Silhouette"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc43_track_matte.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc43_track_matte",
            "Dual-Layer Track Matte",
            "Sử dụng bóng của nhân vật (Target Matte) làm mặt nạ Track Matte để bọc lấy texture không gian Sci-Fi. Hỗ trợ 4 chế độ: Alpha Matte, Inverted Alpha, Luma Matte, Inverted Luma.",
            "Xác thực khả năng đọc đồng thời 2 texture động độc lập và tính toán độ trong suốt Stencil trong Fragment Shader.",
        );
    });
}
