mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    top_color: [f32; 3],
    noise_strength: f32,
    bottom_color: [f32; 3],
    time: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MoonUniform {
    model_view: [f32; 16],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    key_color: [f32; 3],
    tolerance: f32,
    smoothness: f32,
    noise_strength: f32,
    glow_intensity: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudUniform {
    model_view: [f32; 16],
    uv_bounds: [f32; 4],
    key_color_tol: [f32; 4],
    params: [f32; 4],
    lighting_pos: [f32; 4],
}

#[test]
fn run_tc11_viewport() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load Textures
        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_items = h.load_texture("sprites_items.jpeg");
        let tex_props = h.load_texture("bg_nightsky_props.jpeg");

        // 2. Register Pipelines
        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_moon = h.register_moon_pipeline();
        let pipe_cloud = h.register_pipeline("cloud_depth.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_stars = h.register_pipeline(
            "star_particles_sprite.wgsl",
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
            false,
            false,
        );
        let pipe_split = h.register_splitscreen_pipeline();

        // 3. Left Viewport (400x600): Fantasy Heroes Arena
        let left_sky_uni = SkyUniform {
            top_color: [0.18, 0.08, 0.22],
            noise_strength: 0.04,
            bottom_color: [0.08, 0.22, 0.10],
            time: 1.0,
        };
        let bg_left_sky = h.create_custom_uniform_bind_group(left_sky_uni, "Left Sky Uniform");

        // In 400x600 viewport: screen aspect = 400.0 / 600.0 = 0.6666
        let vp_aspect = 400.0f32 / 600.0f32;

        // Paladin Girl Hero (Leftmost in sheet: UV [0.005, 0.01] to [0.28, 0.98])
        let p_scale_y = 0.70f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / vp_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [-0.48, -0.15],
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
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin Uniform");

        // Archer Girl Hero (Third in sheet: UV [0.54, 0.01] to [0.80, 0.98])
        let a_scale_y = 0.68f32;
        let a_crop_w = (0.80 - 0.54) * tex_heroes.width as f32;
        let a_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let a_scale_x = a_scale_y * (a_crop_w / a_crop_h) * (1.0 / vp_aspect);
        let archer_uni = harness::SpriteUniform {
            pos: [0.46, -0.12],
            scale: [a_scale_x, a_scale_y],
            uv_min: [0.54, 0.01],
            uv_max: [0.80, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.6,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_archer = h.create_custom_uniform_bind_group(archer_uni, "Archer Uniform");

        // Golden Chest (Bottom in sprites_items: UV [0.10, 0.52] to [0.42, 0.95])
        let c_scale_y = 0.32f32;
        let c_crop_w = (0.42 - 0.10) * tex_items.width as f32;
        let c_crop_h = (0.95 - 0.52) * tex_items.height as f32;
        let c_scale_x = c_scale_y * (c_crop_w / c_crop_h) * (1.0 / vp_aspect);
        let chest_uni = harness::SpriteUniform {
            pos: [0.0, -0.65],
            scale: [c_scale_x, c_scale_y],
            uv_min: [0.10, 0.52],
            uv_max: [0.42, 0.95],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.4,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_chest = h.create_custom_uniform_bind_group(chest_uni, "Chest Uniform");

        // 4. Right Viewport (400x600): Midnight Celestial Realm
        let right_sky_uni = SkyUniform {
            top_color: [0.008, 0.012, 0.045],
            noise_strength: 0.04,
            bottom_color: [0.025, 0.065, 0.16],
            time: 1.0,
        };
        let bg_right_sky = h.create_custom_uniform_bind_group(right_sky_uni, "Right Sky Uniform");

        // In 400x600 viewport: Height is 1.5x Width, so scale_x must be 1.5x to preserve 1:1 circular moon!
        let vp_inv_aspect = 600.0f32 / 400.0f32;

        // Full Moon in Right Viewport
        let m_scale_y = 0.35f32;
        let m_scale_x = m_scale_y * vp_inv_aspect;
        let moon_pos = [0.0f32, 0.45f32];
        let moon_uni = MoonUniform {
            model_view: [
                m_scale_x, 0.0, 0.0, 0.0,
                0.0, m_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                moon_pos[0], moon_pos[1], 0.0, 1.0,
            ],
            uv_min: [0.39, 0.03],
            uv_max: [0.52, 0.28],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            noise_strength: 0.85,
            glow_intensity: 1.05,
            _pad: 0.0,
        };
        let bg_moon = h.create_custom_uniform_bind_group(moon_uni, "Right Moon Uniform");

        // Wispy Cloud in Right Viewport
        let cloud1_scale_y = 0.24f32;
        let cloud1_scale_x = cloud1_scale_y * (330.0 / 220.0) * vp_inv_aspect;
        let cloud1_uni = CloudUniform {
            model_view: [
                cloud1_scale_x, 0.0, 0.0, 0.0,
                0.0, cloud1_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.15, 0.0, 1.0,
            ],
            uv_bounds: [0.70, 0.34, 0.97, 0.54],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.65, 0.90, 0.90],
            lighting_pos: [moon_pos[0], moon_pos[1], 0.0, 0.15],
        };
        let bg_cloud1 = h.create_custom_uniform_bind_group(cloud1_uni, "Right Cloud 1 Uniform");

        // Cumulus Cloud in Right Viewport
        let cloud2_scale_y = 0.38f32;
        let cloud2_scale_x = cloud2_scale_y * (380.0 / 260.0) * vp_inv_aspect;
        let cloud2_uni = CloudUniform {
            model_view: [
                cloud2_scale_x, 0.0, 0.0, 0.0,
                0.0, cloud2_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, -0.48, 0.0, 1.0,
            ],
            uv_bounds: [0.02, 0.72, 0.36, 0.98],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.15, 0.98, 0.50],
            lighting_pos: [moon_pos[0], moon_pos[1], 0.0, -0.48],
        };
        let bg_cloud2 = h.create_custom_uniform_bind_group(cloud2_uni, "Right Cloud 2 Uniform");

        // 5. Create Offscreen Viewport Targets (400x600 each)
        let (left_target_id, _) = h.create_custom_target(400, 600, "Left Viewport Target");
        let bg_left_view = h.create_texture_bind_group(left_target_id, "Left View");

        let (right_target_id, _) = h.create_custom_target(400, 600, "Right Viewport Target");
        let bg_right_view = h.create_texture_bind_group(right_target_id, "Right View");

        let (final_target_id, final_target_tex) = h.create_target("Final SplitScreen Target");

        // PASS 1: Render Left Viewport (Heroes Realm)
        let mut graph_left = RenderGraph::new(RenderTarget::Offscreen {
            color: left_target_id,
            width: 400,
            height: 600,
        })
        .with_clear_color([0.15, 0.08, 0.20, 1.0]);

        graph_left.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_left_sky, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_archer, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_chest, Vec::new()),
            ],
        );

        // PASS 2: Render Right Viewport (Celestial Night Realm)
        let mut graph_right = RenderGraph::new(RenderTarget::Offscreen {
            color: right_target_id,
            width: 400,
            height: 600,
        })
        .with_clear_color([0.005, 0.008, 0.02, 1.0]);

        graph_right.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_right_sky, Vec::new()),
                DrawCommand::new(pipe_stars, DrawAction::Procedural { vertex_count: 6, instance_range: 0..40 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new()),
                DrawCommand::new(pipe_moon, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, tex_noise.bind_group, Vec::new())
                    .with_bind_group(2, bg_moon, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud1, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud2, Vec::new()),
            ],
        );

        // PASS 3: Split-Screen Final Compositor
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_split, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_left_view, Vec::new())
                    .with_bind_group(1, bg_right_view, Vec::new()),
            ],
        );

        // 6. Execute Passes in Sequence
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_left).expect("Left Viewport failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_right).expect("Right Viewport failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Final SplitScreen failed");

        // 7. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC11 - Multi-Viewport Split-Screen & Camera Isolation",
            "viewports": {
                "left": { "res": "400x600", "scene": "Fantasy Heroes Realm (Paladin, Archer, Chest)" },
                "right": { "res": "400x600", "scene": "Midnight Celestial Realm (Full Moon, Clouds, Stars)" }
            },
            "divider": "Cyan/White Neon Laser Line",
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc11_viewport.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 8. Record Final Output
        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc11_viewport",
            "Multi-Viewport Split-Screen & Camera Isolation",
            "Hai khung cảnh độc lập dựng 100% từ Props được render song song trên 2 Viewport 400x600: Cửa sổ trái là Đấu trường Anh hùng (Paladin, Archer, Chest), Cửa sổ phải là Bầu trời Đêm Trăng tròn. Cả hai được ghép đối xứng qua đường viền laser rực rỡ không hề có hiện tượng rò rỉ trạng thái hay méo hình.",
            "Xác thực năng lực đa camera (Multi-Camera Viewports) và đa RenderTarget độc lập của ifol-gpu. Tỉ lệ khung hình của từng prop được bảo toàn hoàn hảo ở từng khung nhìn riêng biệt.",
        );
    });
}
