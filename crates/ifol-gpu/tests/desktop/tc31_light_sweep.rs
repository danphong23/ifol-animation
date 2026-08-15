mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightSweepUniform {
    progress: f32,
    angle: f32, // in radians
    width: f32,
    intensity: f32,
    color: [f32; 3],
    _pad: f32,
}

#[test]
fn run_tc31_light_sweep() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_sweep = h.register_pipeline("light_sweep.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Paladin (Mage for variety? Let's use Mage)
        let m_scale_y = 0.80f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.0, 0.0],
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

        let sweep_uni = LightSweepUniform {
            progress: 0.6, // Sweeping midway
            angle: std::f32::consts::PI / 4.0, // 45 degrees
            width: 0.15,
            intensity: 2.0,
            color: [1.0, 1.0, 0.8], // bright warm glow
            _pad: 0.0,
        };
        let bg_sweep_uni = h.create_custom_uniform_bind_group(sweep_uni, "Sweep Uniform");

        let (target_a_id, _target_a_tex) = h.create_target("Target A");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_a = h.create_texture_bind_group(target_a_id, "Sweep Texture BG");

        // Pass 1: Extract Mage via Chroma Key to transparent offscreen target
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
                    .with_bind_group(1, bg_mage, Vec::new()),
            ],
        );

        // Pass 2: Apply Light Sweep over dark gray background
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.15, 0.15, 0.15, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sweep, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_a, Vec::new())
                    .with_bind_group(1, bg_sweep_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC31 - Light Sweep (Shine Effect)",
            "features": [
                "Math-based Diagonal Sweep",
                "Distance tracking in UV space",
                "Additive Blending"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc31_light_sweep.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc31_light_sweep",
            "Light Sweep (Shine) Effect",
            "Hiệu ứng luồng sáng xiên quét ngang qua nhân vật. Sử dụng toán học để quét vùng sáng 45 độ.",
            "Test khả năng tính toán đường chéo và Additive Blending kết hợp giữ nguyên Alpha của nhân vật.",
        );
    });
}
