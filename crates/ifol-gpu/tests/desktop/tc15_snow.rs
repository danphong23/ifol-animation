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
struct ParticleSimUniform {
    time: f32,
    wind_speed: f32,
    gravity: f32,
    particle_count: f32,
}

#[test]
fn run_tc15_snow() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load Textures
        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_props = h.load_texture("bg_nightsky_props.jpeg");
        let tex_forest = h.load_texture("bg_forest_props1.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_snow = h.load_texture("particle_snow.jpeg");

        // 2. Register Pipelines
        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_moon = h.register_moon_pipeline();
        let pipe_cloud = h.register_pipeline("cloud_depth.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_snow = h.register_pipeline("snow_physics_instanced.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        let screen_aspect = 800.0f32 / 600.0f32;

        // 3. Icy Winter Midnight Sky
        let sky_uni = SkyUniform {
            top_color: [0.01, 0.02, 0.08], // Deep freezing indigo
            noise_strength: 0.04,
            bottom_color: [0.05, 0.12, 0.25], // Crisp icy twilight
            time: 2.5,
        };
        let bg_sky = h.create_custom_uniform_bind_group(sky_uni, "Winter Sky Uniform");

        // Radiant Full Moon (Top-Left)
        let moon_pos = [-0.42, 0.45];
        let m_scale_y = 0.36f32;
        let m_scale_x = m_scale_y * (1.0 / screen_aspect);
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
        let bg_moon = h.create_custom_uniform_bind_group(moon_uni, "Moon Uniform");

        // Wispy Cloud drifting near Moon
        let c1_center = [-0.15, 0.22];
        let c1_scale_y = 0.26f32;
        let c1_scale_x = c1_scale_y * (330.0 / 220.0) * (1.0 / screen_aspect);
        let cloud1_uni = CloudUniform {
            model_view: [
                c1_scale_x, 0.0, 0.0, 0.0,
                0.0, c1_scale_y, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                c1_center[0], c1_center[1], 0.0, 1.0,
            ],
            uv_bounds: [0.70, 0.34, 0.97, 0.54],
            key_color_tol: [0.0, 1.0, 0.0, 0.48],
            params: [0.10, 0.65, 0.90, 0.90],
            lighting_pos: [moon_pos[0], moon_pos[1], c1_center[0], c1_center[1]],
        };
        let bg_cloud1 = h.create_custom_uniform_bind_group(cloud1_uni, "Cloud 1 Uniform");

        // Winter Pine Trees (Left & Right)
        let t1_scale_y = 0.72f32;
        let t1_crop_w = (0.70 - 0.58) * tex_forest.width as f32;
        let t1_crop_h = (0.42 - 0.02) * tex_forest.height as f32;
        let t1_scale_x = t1_scale_y * (t1_crop_w / t1_crop_h) * (1.0 / screen_aspect);
        let pine1_uni = harness::SpriteUniform {
            pos: [-0.62, 0.15],
            scale: [t1_scale_x, t1_scale_y],
            uv_min: [0.58, 0.02],
            uv_max: [0.70, 0.42],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.7,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_pine1 = h.create_custom_uniform_bind_group(pine1_uni, "Pine 1 Uniform");

        let pine2_uni = harness::SpriteUniform {
            pos: [0.62, 0.15],
            scale: [t1_scale_x, t1_scale_y],
            uv_min: [0.58, 0.02],
            uv_max: [0.70, 0.42],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.7,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_pine2 = h.create_custom_uniform_bind_group(pine2_uni, "Pine 2 Uniform");

        // Foreground Paladin Hero in Snow
        let p_scale_y = 0.62f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [0.0, -0.22],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.4,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin Uniform");

        // 4. Snowflake Physics Simulation Uniform (Time = 3.5s, wind = 1.2, gravity = 0.8)
        let snow_sim_uni = ParticleSimUniform {
            time: 3.5,
            wind_speed: 1.2,
            gravity: 0.8,
            particle_count: 200.0,
        };
        let bg_snow_sim = h.create_custom_uniform_bind_group(snow_sim_uni, "Snow Sim Uniform");

        // 5. Output Target
        let (target_id, target_tex) = h.create_target("TC15 Snow Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.01, 0.02, 0.06, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_sky, Vec::new()),
                DrawCommand::new(pipe_moon, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, tex_noise.bind_group, Vec::new())
                    .with_bind_group(2, bg_moon, Vec::new()),
                DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new())
                    .with_bind_group(1, bg_cloud1, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_pine1, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_pine2, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                // 200 Falling Animated Snowflakes with Physics
                DrawCommand::new(pipe_snow, DrawAction::Procedural { vertex_count: 6, instance_range: 0..200 })
                    .with_bind_group(0, tex_snow.bind_group, Vec::new())
                    .with_bind_group(1, bg_snow_sim, Vec::new()),
            ],
        );

        // 6. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC15 - Animated Particle Physics Simulator (Winter Snow Scene)",
            "particle_system": {
                "instance_count": 200,
                "physics": ["Gravity fall", "Sinusoidal wind drift", "Individual spin rotation", "Depth-based size & opacity"],
                "shader": "snow_physics_instanced.wgsl"
            },
            "environment_props": [
                "Icy Midnight Procedural Gradient Sky",
                "Full Moon with Lunar Maria & Limb Glow",
                "Wispy Cloud with Silver Lining",
                "Winter Pine Trees",
                "Paladin Hero in Winter Realm"
            ],
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc15_snow.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 7. Record Output
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc15_snow",
            "Animated Particle Physics Simulator (Winter Snow Scene)",
            "200 hạt tuyết rơi chuyển động vật lý (trọng lực, gió tạt, xoay cánh tuyết, phân tầng xa gần) được mô phỏng mượt mà trên khung cảnh đêm tuyết mùa đông dựng hoàn toàn từ Props (Cây thông, Nữ hiệp sĩ, Trăng rằm, Mây).",
            "Xác thực năng lực Instanced Particle Physics Simulation trên GPU của ifol-gpu. Hoàn thành kiểm tra chuyển động hạt động thời gian thực.",
        );
    });
}
