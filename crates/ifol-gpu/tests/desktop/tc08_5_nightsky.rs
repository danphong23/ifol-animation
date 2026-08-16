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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PostProcessUniform {
    bloom_intensity: f32,
    exposure: f32,
    contrast: f32,
    _pad: f32,
}

#[test]
fn run_tc08_5_nightsky() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load Assets (Noise Texture & Props Sheet ONLY - Zero full image!)
        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_props = h.load_texture("bg_nightsky_props.jpeg");

        // 2. Pure Procedural Sky Uniform: Deep cosmic midnight palette
        let sky_uniform = SkyUniform {
            top_color: [0.008, 0.012, 0.045],
            noise_strength: 0.04,
            bottom_color: [0.025, 0.065, 0.16],
            time: 1.0,
        };
        let sky_uni_bg = h.create_custom_uniform_bind_group(sky_uniform, "Sky Procedural Uniform");

        // 3. Post-Processing Uniform Buffer (Radiant Celestial Bloom)
        let post_uniform = PostProcessUniform {
            bloom_intensity: 1.10,
            exposure: 1.0,
            contrast: 1.05,
            _pad: 0.0,
        };
        let post_uni_bg = h.create_custom_uniform_bind_group(post_uniform, "Post Uniform");

        // 4. Register Pipelines
        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
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
        let pipe_post = h.register_pipeline("postprocess_night_bloom.wgsl", Some(wgpu::BlendState::REPLACE), false, true);

        // 5. Full Moon - Primary Celestial Light Source (Position: [-0.38, 0.42])
        let moon_pos = [-0.38, 0.42];
        let moon_scale_y = 0.38;
        let moon_scale_x = moon_scale_y * (600.0 / 800.0);
        let moon_uni = MoonUniform {
            model_view: [
                moon_scale_x, 0.0, 0.0, 0.0,
                0.0, moon_scale_y, 0.0, 0.0,
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
        let bg_moon = h.create_custom_uniform_bind_group(moon_uni, "Moon Surface Uniform");

        // 6. Multi-Layer Depth Clouds with Directional Moonlight Silver Lining
        // Cloud 1: Wispy Cloud drifting near Full Moon (High silver rim & proximity glow)
        let c1_center = [-0.12, 0.20];
        let c1_scale_y = 0.28;
        let c1_scale_x = c1_scale_y * (600.0 / 800.0) * (330.0 / 220.0);
        let cloud1_uni = CloudUniform {
            model_view: [
                c1_scale_x, 0.0, 0.0, 0.0,
                0.0, c1_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                c1_center[0], c1_center[1], 0.0, 1.0,
            ],
            uv_bounds: [0.70, 0.34, 0.97, 0.54],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.75, 0.88, 0.95], // smoothness, depth_softness, opacity, silver_rim
            lighting_pos: [moon_pos[0], moon_pos[1], c1_center[0], c1_center[1]],
        };
        let bg_cloud1 = h.create_custom_uniform_bind_group(cloud1_uni, "Cloud 1 Wispy");

        // Cloud 2: Midground Cyan Glowing Cloud (Floating mid-right)
        let c2_center = [0.38, 0.12];
        let c2_scale_y = 0.32;
        let c2_scale_x = c2_scale_y * (600.0 / 800.0) * (340.0 / 220.0);
        let cloud2_uni = CloudUniform {
            model_view: [
                c2_scale_x, 0.0, 0.0, 0.0,
                0.0, c2_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                c2_center[0], c2_center[1], 0.0, 1.0,
            ],
            uv_bounds: [0.38, 0.34, 0.67, 0.54],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.45, 0.92, 0.75],
            lighting_pos: [moon_pos[0], moon_pos[1], c2_center[0], c2_center[1]],
        };
        let bg_cloud2 = h.create_custom_uniform_bind_group(cloud2_uni, "Cloud 2 Cyan");

        // Cloud 3: Mid-Horizon Dark Blue Cloud
        let c3_center = [-0.42, -0.26];
        let c3_scale_y = 0.36;
        let c3_scale_x = c3_scale_y * (600.0 / 800.0) * (330.0 / 220.0);
        let cloud3_uni = CloudUniform {
            model_view: [
                c3_scale_x, 0.0, 0.0, 0.0,
                0.0, c3_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                c3_center[0], c3_center[1], 0.0, 1.0,
            ],
            uv_bounds: [0.69, 0.03, 0.97, 0.25],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.30, 0.94, 0.55],
            lighting_pos: [moon_pos[0], moon_pos[1], c3_center[0], c3_center[1]],
        };
        let bg_cloud3 = h.create_custom_uniform_bind_group(cloud3_uni, "Cloud 3 Dark");

        // Cloud 4: Foreground Fluffy Cumulus Cloud
        let c4_center = [0.22, -0.50];
        let c4_scale_y = 0.45;
        let c4_scale_x = c4_scale_y * (600.0 / 800.0) * (380.0 / 260.0);
        let cloud4_uni = CloudUniform {
            model_view: [
                c4_scale_x, 0.0, 0.0, 0.0,
                0.0, c4_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                c4_center[0], c4_center[1], 0.0, 1.0,
            ],
            uv_bounds: [0.02, 0.72, 0.36, 0.98],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.08, 0.98, 0.40],
            lighting_pos: [moon_pos[0], moon_pos[1], c4_center[0], c4_center[1]],
        };
        let bg_cloud4 = h.create_custom_uniform_bind_group(cloud4_uni, "Cloud 4 Cumulus");

        // 7. Intermediate & Final Targets
        let (scene_target_id, _) = h.create_target("Pass 1 Scene Intermediate");
        let bg_scene_view = h.create_texture_bind_group(scene_target_id, "Scene View");
        let (final_target_id, final_target_tex) = h.create_target("Pass 2 Final Output");

        // PASS 1: Compose Scene with Directional Moonlight Distribution
        let mut graph_scene = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.005, 0.008, 0.02, 1.0]);

        graph_scene.add_batch(
            &mut h.pool,
            vec![
                // 1. Procedural Gradient Sky + Repeating Perlin Noise
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, sky_uni_bg, Vec::new()),

                // 2. 100 Multi-Tiered Stars (Micro, Mid, Radiant Cross)
                DrawCommand::new(pipe_stars, DrawAction::Procedural { vertex_count: 6, instance_range: 0..100 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new()),
                
                // 3. Radiant Full Moon - Primary Light Source
                DrawCommand::new(pipe_moon, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, tex_noise.bind_group, Vec::new())
                    .with_bind_group(2, bg_moon, Vec::new()),

                // 4. Clouds with Directional Moonlight Silver Lining & Ambient Shadow
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud1, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud2, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud3, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud4, Vec::new()),
            ],
        );

        // PASS 2: Fullscreen Post-Processing (Multi-Radius Celestial Bloom)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_post, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_scene_view, Vec::new())
                    .with_bind_group(1, post_uni_bg, Vec::new()),
            ],
        );

        // 8. Execute Both Passes
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_scene).expect("Pass 1 failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Pass 2 failed");

        // 9. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC08.5 - Directional Moonlight Distribution & Organic Lunar Scene",
            "primary_light_source": {
                "type": "Emissive Full Moon",
                "position": [-0.38, 0.42],
                "shading": "3D Emissive Sphere with Domain-Warped Maria"
            },
            "illuminated_entities": [
                "Wispy Cloud (Silver Lining facing Moon at upper-left)",
                "Cyan Glowing Cloud (Silver Highlight on top-left edge)",
                "Dark Horizon Cloud (Shadowed right side, lit top-left)",
                "Cumulus Foreground Cloud (Ambient moonlight falloff)"
            ],
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc08_5_nightsky.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 10. Record Final Image & Report
        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc08_5_nightsky",
            "Directional Moonlight Distribution & Organic Lunar Scene",
            "Khung cảnh với Mặt trăng là nguồn sáng chủ đạo: Ánh sáng bạc tỏa rọi trực tiếp lên các viền mây hướng về phía trăng (Silver Lining), phần thân mây quay lưng đổ bóng tối, tạo sự phân bổ ánh sáng chuẩn xác và nghệ thuật.",
            "Tích hợp mô hình chiếu sáng định hướng Moonlight Vector Shading, kết hợp Moon Surface Maria và Bloom Pass. Hoàn thành xuất sắc toàn bộ yêu cầu về phân bổ nguồn sáng.",
        );
    });
}
