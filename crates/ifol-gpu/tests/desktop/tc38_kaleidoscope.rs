mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct KaleidoscopeUniform {
    segments: f32, // number of slices
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[test]
fn run_tc38_kaleidoscope() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_kaleido = h.register_pipeline("kaleidoscope.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Mage
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

        let kaleido_uni = KaleidoscopeUniform {
            segments: 6.0, // hexagon mirror
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let bg_kaleido_uni = h.create_custom_uniform_bind_group(kaleido_uni, "Kaleidoscope Uniform");

        let (target_a_id, _target_a_tex) = h.create_target("Target A");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_a = h.create_texture_bind_group(target_a_id, "Kaleido Texture BG");

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

        // Pass 2: Apply Kaleidoscope over a mystic background
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.2, 0.0, 0.4, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_kaleido, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_a, Vec::new())
                    .with_bind_group(1, bg_kaleido_uni, Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC38 - Kaleidoscope",
            "features": [
                "Polar Coordinate mapping",
                "Angular Modulo Folding"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc38_kaleidoscope.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc38_kaleidoscope",
            "Kaleidoscope Mirror Filter",
            "Biến dạng hình ảnh theo phong cách kính vạn hoa bằng cách chuyển hệ tọa độ Descartes sang hệ tọa độ Cực (Polar Coordinates).",
            "Xác thực phép gập góc (Angular fold) bằng cách sử dụng modulo và abs trên 6 phân đoạn (segments).",
        );
    });
}
