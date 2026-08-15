mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct KawaseUniform {
    offset: f32,
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
}

#[test]
fn run_tc55_dual_kawase() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_kawase = h.register_pipeline("dual_kawase.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_additive = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        }), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

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

        let kawase_uni = KawaseUniform {
            offset: 4.5,
            intensity: 2.2,
            _pad0: 0.0,
            _pad1: 0.0,
        };
        let bg_kawase_uni = h.create_custom_uniform_bind_group(kawase_uni, "Kawase Uniform");

        let (target_mage_id, _target_mage_tex) = h.create_target("Target Mage");
        let (target_downsample_id, _target_down_tex) = h.create_custom_target(400, 300, "Target Downsample");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_mage = h.create_texture_bind_group(target_mage_id, "Mage Texture BG");
        let bg_tex_down = h.create_texture_bind_group(target_downsample_id, "Downsample Texture BG");

        // Pass 1: Extract Mage via Chroma Key (800x600)
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

        // Pass 2: Downsample & 8-Tap Kawase Blur into 400x300 Low-Res Target
        let mut graph_down = RenderGraph::new(RenderTarget::Offscreen {
            color: target_downsample_id,
            width: 400,
            height: 300,
        }).with_clear_color([0.0, 0.0, 0.0, 0.0]);

        graph_down.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_kawase, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_mage.clone(), Vec::new())
                    .with_bind_group(1, bg_kawase_uni.clone(), Vec::new()),
            ],
        );

        // Pass 3: Final Composite (Sci-Fi BG + Additive Kawase Bloom + Sharp Mage)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.05, 0.05, 0.1, 1.0]);

        // Background
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_screen, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_kawase_uni.clone(), Vec::new()),
            ],
        );

        // Additive Kawase Bloom (Upsampled from 400x300)
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_additive, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_down, Vec::new())
                    .with_bind_group(1, bg_kawase_uni.clone(), Vec::new()),
            ],
        );

        // Sharp Mage on top
        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_screen, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_mage, Vec::new())
                    .with_bind_group(1, bg_kawase_uni.clone(), Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_down).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC55 - Dual Kawase Fast Bloom & Downsample Hierarchy",
            "features": [
                "Hierarchical Downsampling (800x600 -> 400x300)",
                "8-Tap Dual Kawase Bloom Filter",
                "High-Speed 60FPS Additive Glow Upsampling"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc55_dual_kawase.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc55_dual_kawase",
            "Dual Kawase Bloom Filter",
            "Thuật toán làm mờ phân cấp Dual Kawase Blur: Giảm kích thước khung hình xuống 400x300 và lấy mẫu 8 điểm đa hướng, sau đó phóng to cộng dồn màu quang học lên khung hình gốc đạt tốc độ xử lý siêu nhanh vượt trội.",
            "Xác thực luồng Render Graph phân cấp đa độ phân giải (Hierarchical Multi-Resolution Target Flow).",
        );
    });
}
