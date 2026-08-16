mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BloomUniform {
    threshold: f32,
    intensity: f32,
    blur_radius: f32,
    _pad: f32,
}

#[test]
fn run_tc42_hdr_bloom() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_bloom = h.register_pipeline("emissive_bloom.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_additive = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        }), false, true);
        let pipe_alpha_over = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Mage in center
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

        let bloom_uni = BloomUniform {
            threshold: 0.2,
            intensity: 2.0,
            blur_radius: 5.0,
            _pad: 0.0,
        };
        let bg_bloom_uni = h.create_custom_uniform_bind_group(bloom_uni, "Bloom Uniform");

        let (target_mage_id, _target_mage_tex) = h.create_target("Target Mage");
        let (target_bloom_id, _target_bloom_tex) = h.create_target("Target Bloom");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_mage = h.create_texture_bind_group(target_mage_id, "Mage Texture BG");
        let bg_tex_bloom = h.create_texture_bind_group(target_bloom_id, "Bloom Texture BG");

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
                    .with_bind_group(1, bg_mage, Vec::new()),
            ],
        );

        // Pass 2: Extract Emissive Glow from Target Mage across whole 800x600 screen into Target Bloom
        let mut graph_bloom = RenderGraph::new(RenderTarget::Offscreen {
            color: target_bloom_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 0.0]);

        graph_bloom.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_bloom, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_mage.clone(), Vec::new())
                    .with_bind_group(1, bg_bloom_uni, Vec::new()),
            ],
        );

        // Pass 3: Final Composite (Sci-Fi Background + Additive Bloom + Sharp Mage)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.05, 0.05, 0.1, 1.0]);

        // Background
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_alpha_over, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_bloom_uni.clone(), Vec::new()),
            ],
        );

        // Additive Bloom (spills widely over the whole screen)
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_additive, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_bloom, Vec::new())
                    .with_bind_group(1, bg_bloom_uni.clone(), Vec::new()),
            ],
        );

        // Sharp Mage on top
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_alpha_over, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_mage, Vec::new())
                    .with_bind_group(1, bg_bloom_uni.clone(), Vec::new()),
            ],
        );

        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_bloom).expect("Execution failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC42 - Full-Frame HDR Bloom & Emissive Glow",
            "features": [
                "Full-Screen Emissive Thresholding",
                "Wide Radial Gaussian Dispersion",
                "Additive Optical Composite (No Mesh Edge Clipping)"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc42_hdr_bloom.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc42_hdr_bloom",
            "Full-Frame HDR Bloom & Glow",
            "Khắc phục triệt để lỗi Glow bị cắt vuông ở viền Sprite: Tách lớp phát sáng ra toàn khung hình (800x600), áp dụng bộ lọc Wide Gaussian Blur và cộng quang học (Additive Blending) lên nền Sci-Fi.",
            "Xác thực luồng Multi-Pass Composite 3 giai đoạn: Isolate -> Screen Blur -> Additive Blend.",
        );
    });
}
