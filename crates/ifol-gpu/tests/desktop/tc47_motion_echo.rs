mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct EchoUniform {
    velocity: [f32; 2],
    decay: f32,
    hue_shift: f32,
    num_echoes: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[test]
fn run_tc47_motion_echo() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_echo = h.register_pipeline("motion_echo.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Mage moving rightwards
        let m_scale_y = 0.75f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.15, 0.0],
            scale: [m_scale_x, m_scale_y],
            uv_min: [0.27, 0.01],
            uv_max: [0.54, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage");

        let echo_uni = EchoUniform {
            velocity: [-0.05, 0.0], // Motion trailing to the left
            decay: 0.65,
            hue_shift: 1.2,
            num_echoes: 5.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let bg_echo_uni = h.create_custom_uniform_bind_group(echo_uni, "Echo Uniform");

        let (target_mage_id, _target_mage_tex) = h.create_target("Target Mage");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_mage = h.create_texture_bind_group(target_mage_id, "Mage Texture BG");

        // Pass 1: Extract Mage via Chroma Key into transparent Target Mage (800x600)
        let mut graph_chroma = RenderGraph::new(RenderTarget::Offscreen {
            color: target_mage_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 0.0]);

        graph_chroma.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_mage.clone(), Vec::new()),
            ],
        );

        // Pass 2: Final Composite (Sci-Fi Background + Motion Echo Trail)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_screen, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_echo_uni.clone(), Vec::new()),
                DrawCommand::new(pipe_echo, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_mage, Vec::new())
                    .with_bind_group(1, bg_echo_uni, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC47 - Motion Echo & Afterimage Ghosting",
            "features": [
                "Multi-Step Temporal Offset Sampling",
                "Exponential Alpha Attenuation",
                "Chromatic Trail Phase Shifting"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc47_motion_echo.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc47_motion_echo",
            "Motion Echo & Afterimage",
            "Hiệu ứng tàn ảnh di chuyển tốc độ cao (Speed Dash Afterimage): Lưu lại 5 lớp bóng ma của Pháp Sư với độ giảm mờ lũy thừa (Decay) và xoay chuyển sắc màu (Spectral Hue Trail) trên nền Sci-Fi.",
            "Xác thực kỹ thuật tổng hợp chuỗi tàn ảnh đa tầng (Multi-layer temporal composite) trong Fragment Shader.",
        );
    });
}
