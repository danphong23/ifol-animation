mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct OutlineUniform {
    outline_color: [f32; 4],
    shadow_color: [f32; 4],
    shadow_offset: [f32; 2],
    texel_size: [f32; 2],
    outline_thickness: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    top_color: [f32; 3],
    noise_strength: f32,
    bottom_color: [f32; 3],
    time: f32,
}

#[test]
fn run_tc17_outline() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_items = h.load_texture("sprites_items.jpeg");

        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        
        let pipe_outline = h.register_pipeline(
            "outline_shadow.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let screen_aspect = 800.0f32 / 600.0f32;

        // Background Sky
        let sky_uni = SkyUniform {
            top_color: [0.15, 0.08, 0.25], // Purple
            noise_strength: 0.02,
            bottom_color: [0.4, 0.15, 0.3], // Magenta
            time: 1.0,
        };
        let bg_sky = h.create_custom_uniform_bind_group(sky_uni, "Sky Uniform");

        // Foreground Heroes (Rendered to Transparent Target A)
        // Paladin (Left)
        let p_scale_y = 0.70f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [-0.35, -0.1],
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

        // Mage (Right)
        let m_scale_y = 0.70f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.35, -0.1],
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

        // Chest (Center)
        let c_scale_y = 0.35f32;
        let c_crop_w = (0.42 - 0.10) * tex_items.width as f32;
        let c_crop_h = (0.95 - 0.52) * tex_items.height as f32;
        let c_scale_x = c_scale_y * (c_crop_w / c_crop_h) * (1.0 / screen_aspect);
        let chest_uni = harness::SpriteUniform {
            pos: [0.0, -0.4],
            scale: [c_scale_x, c_scale_y],
            uv_min: [0.10, 0.52],
            uv_max: [0.42, 0.95],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.2,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_chest = h.create_custom_uniform_bind_group(chest_uni, "Chest");

        // Outline Uniform (Thick White Outline + Deep Black Shadow)
        let outline_uni = OutlineUniform {
            outline_color: [1.0, 1.0, 1.0, 1.0], // Solid White
            shadow_color: [0.0, 0.0, 0.0, 0.7],  // Black 70% opacity
            shadow_offset: [-0.03, 0.03],        // Offset down and right
            texel_size: [1.0 / 800.0, 1.0 / 600.0],
            outline_thickness: 3.5, // Pixels
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        let bg_outline = h.create_custom_uniform_bind_group(outline_uni, "Outline FX");

        // Targets
        let (transparent_target_id, _) = h.create_target("Transparent Heroes");
        let bg_transparent_view = h.create_texture_bind_group(transparent_target_id, "Heroes Texture View");
        
        let (final_target_id, final_target_tex) = h.create_target("TC17 Final Target");

        // Pass 1: Render Heroes to Transparent Target
        let mut graph_heroes = RenderGraph::new(RenderTarget::Offscreen {
            color: transparent_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]); // Fully Transparent

        graph_heroes.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_mage, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_chest, Vec::new()),
            ],
        );

        // Pass 2: Render Sky, then Outline Post-Process over it
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                // 1. Draw Sky Background
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_sky, Vec::new()),
                
                // 2. Draw Heroes with Outline & Shadow Post-Processing Filter
                DrawCommand::new(pipe_outline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_transparent_view, Vec::new())
                    .with_bind_group(1, bg_outline, Vec::new()),
            ],
        );

        // Execute
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_heroes).expect("Pass 1 failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Pass 2 failed");

        let graph_json = serde_json::json!({
            "test_case": "TC17 - Multi-Pass Outline Stroke & Drop Shadow Filter",
            "features": [
                "Offscreen Transparent Target Rendering",
                "8-way discrete pixel sampling for edge detection",
                "Procedural Stroke/Outline Generation",
                "Alpha-composited Drop Shadow offset"
            ],
            "passes": 2
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc17_outline.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc17_outline",
            "Multi-Pass Outline Stroke & Drop Shadow Filter",
            "Hiệu ứng Stroke bọc viền trắng và bóng đổ đen (Drop Shadow) kinh điển của Motion Graphics. Các nhân vật (Paladin, Mage, Rương) được render vào một layer trong suốt trước, sau đó bộ lọc hậu kỳ (Post-processing) sẽ dò tìm vùng biên Alpha để vẽ viền và bóng, sau cùng mới in lên nền bầu trời.",
            "Xác thực năng lực Post-processing Masking và Edge Detection bằng GPU.",
        );
    });
}
