mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AtlasUniform {
    pos: [f32; 2],
    scale: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    texture_size: [f32; 2],
    enable_clamp: f32,
    tolerance: f32,
    smoothness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    key_color: [f32; 4],
}

#[test]
fn run_tc51_atlas_clamp() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_atlas = h.register_pipeline("atlas_clamp.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_screen = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Paladin (Left) - with Half-Texel Inset Clamp
        let p_scale_y = 0.82f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = AtlasUniform {
            pos: [-0.40, 0.0],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            texture_size: [tex_heroes.width as f32, tex_heroes.height as f32],
            enable_clamp: 1.0,
            tolerance: 0.48,
            smoothness: 0.10,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            key_color: [0.0, 1.0, 0.0, 1.0],
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin");

        // Mage (Right) - adjacent to Paladin in texture atlas, also clamped
        let m_scale_y = 0.82f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = AtlasUniform {
            pos: [0.40, 0.0],
            scale: [m_scale_x, m_scale_y],
            uv_min: [0.27, 0.01],
            uv_max: [0.54, 0.98],
            texture_size: [tex_heroes.width as f32, tex_heroes.height as f32],
            enable_clamp: 1.0,
            tolerance: 0.48,
            smoothness: 0.10,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            key_color: [0.0, 1.0, 0.0, 1.0],
        };
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_screen, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin.clone(), Vec::new()),
                DrawCommand::new(pipe_atlas, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_atlas, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_mage, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC51 - Texture Atlas Sub-pixel Bleed Prevention",
            "features": [
                "Half-Texel Boundary Inset Clamping",
                "Sub-pixel Bilinear Filtering Guard",
                "Side-by-side Atlas Sprite Isolation"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc51_atlas_clamp.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &final_target_tex,
            "tc51_atlas_clamp",
            "Texture Atlas Bleed Prevention",
            "Kỹ thuật kẹp nửa Texel (Half-Texel UV Inset Clamping) ngăn ngừa hiện tượng lem viền (Color Bleeding) giữa các Sprite đứng sát nhau trên cùng một tấm Texture Atlas khi nội suy Linear Filter.",
            "Xác thực việc render song song Paladin và Pháp Sư được cắt từ cùng một Sprite Sheet với biên giới sắc nét tuyệt đối.",
        );
    });
}
