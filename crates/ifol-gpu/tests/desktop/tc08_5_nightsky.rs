mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

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

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn address_mode(value: &str) -> wgpu::AddressMode {
    match value {
        "repeat" => wgpu::AddressMode::Repeat,
        "mirror-repeat" => wgpu::AddressMode::MirrorRepeat,
        "clamp-to-edge" => wgpu::AddressMode::ClampToEdge,
        other => panic!("Unsupported TC08.5 sampler address mode: {other}"),
    }
}

fn filter_mode(value: &str) -> wgpu::FilterMode {
    match value {
        "nearest" => wgpu::FilterMode::Nearest,
        "linear" => wgpu::FilterMode::Linear,
        other => panic!("Unsupported TC08.5 sampler filter mode: {other}"),
    }
}

fn mipmap_filter_mode(value: &str) -> wgpu::MipmapFilterMode {
    match value {
        "nearest" => wgpu::MipmapFilterMode::Nearest,
        "linear" => wgpu::MipmapFilterMode::Linear,
        other => panic!("Unsupported TC08.5 sampler mipmap filter mode: {other}"),
    }
}

#[test]
fn run_tc08_5_nightsky() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc08_5_nightsky.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC08.5 manifest");
        let graph_spec = &manifest["graph"];
        assert_eq!(graph_spec["passes"].as_array().unwrap().len(), 2);
        assert_eq!(graph_spec["command_count"], 8);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let sampler_spec = &graph_spec["sampler"];
        h.sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: address_mode(sampler_spec["address_mode_u"].as_str().unwrap()),
            address_mode_v: address_mode(sampler_spec["address_mode_v"].as_str().unwrap()),
            address_mode_w: address_mode(sampler_spec["address_mode_w"].as_str().unwrap()),
            mag_filter: filter_mode(sampler_spec["mag_filter"].as_str().unwrap()),
            min_filter: filter_mode(sampler_spec["min_filter"].as_str().unwrap()),
            mipmap_filter: mipmap_filter_mode(sampler_spec["mipmap_filter"].as_str().unwrap()),
            ..Default::default()
        });

        let tex_noise = h.load_texture("canonical_tc085_noise.png");
        let tex_props = h.load_texture("canonical_tc085_props.png");

        let sky_uni_bg = h.create_custom_uniform_bind_group(SkyUniform {
            top_color: [0.008, 0.012, 0.045],
            noise_strength: 0.04,
            bottom_color: [0.025, 0.065, 0.16],
            time: 1.0,
        }, "TC08.5 Sky Uniform");
        let post_uni_bg = h.create_custom_uniform_bind_group(PostProcessUniform {
            bloom_intensity: 1.10,
            exposure: 1.0,
            contrast: 1.05,
            _pad: 0.0,
        }, "TC08.5 Post Uniform");

        let pipe_sky = h.register_sky_pipeline();
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

        let moon_uni = h.create_custom_uniform_bind_group(MoonUniform {
            model_view: [0.285, 0.0, 0.0, 0.0, 0.0, 0.38, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -0.38, 0.42, 0.0, 1.0],
            uv_min: [0.39, 0.03],
            uv_max: [0.52, 0.28],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            noise_strength: 0.85,
            glow_intensity: 1.05,
            _pad: 0.0,
        }, "TC08.5 Moon Uniform");

        let cloud_uniform = |h: &mut DesktopTestHarness<'_>, model_view, uv_bounds, params, lighting_pos, label| {
            h.create_custom_uniform_bind_group(CloudUniform {
                model_view,
                uv_bounds,
                key_color_tol: [0.0, 1.0, 0.0, 0.48],
                params,
                lighting_pos,
            }, label)
        };
        let cloud1 = cloud_uniform(&mut h, [0.315, 0.0, 0.0, 0.0, 0.0, 0.28, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -0.12, 0.20, 0.0, 1.0], [0.70, 0.34, 0.97, 0.54], [0.10, 0.75, 0.88, 0.95], [-0.38, 0.42, -0.12, 0.20], "TC08.5 Cloud 1");
        let cloud2 = cloud_uniform(&mut h, [0.3709091, 0.0, 0.0, 0.0, 0.0, 0.32, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.38, 0.12, 0.0, 1.0], [0.38, 0.34, 0.67, 0.54], [0.10, 0.45, 0.92, 0.75], [-0.38, 0.42, 0.38, 0.12], "TC08.5 Cloud 2");
        let cloud3 = cloud_uniform(&mut h, [0.405, 0.0, 0.0, 0.0, 0.0, 0.36, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -0.42, -0.26, 0.0, 1.0], [0.69, 0.03, 0.97, 0.25], [0.10, 0.30, 0.94, 0.55], [-0.38, 0.42, -0.42, -0.26], "TC08.5 Cloud 3");
        let cloud4 = cloud_uniform(&mut h, [0.4932692, 0.0, 0.0, 0.0, 0.0, 0.45, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.22, -0.50, 0.0, 1.0], [0.02, 0.72, 0.36, 0.98], [0.10, 0.08, 0.98, 0.40], [-0.38, 0.42, 0.22, -0.50], "TC08.5 Cloud 4");

        let (scene_target_id, _) = h.create_target("TC08.5 Scene Target");
        let scene_view = h.create_texture_bind_group(scene_target_id, "TC08.5 Scene View");
        let (final_target_id, final_target_tex) = h.create_target("TC08.5 Final Target");
        let mut scene_graph = RenderGraph::new(RenderTarget::Offscreen { color: scene_target_id, width: 800, height: 600 })
            .with_clear_color([0.005, 0.008, 0.02, 1.0]);
        scene_graph.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_noise.bind_group, Vec::new()).with_bind_group(1, sky_uni_bg, Vec::new()),
            DrawCommand::new(pipe_stars, DrawAction::Procedural { vertex_count: 6, instance_range: 0..100 }).with_bind_group(0, tex_props.bind_group, Vec::new()),
            DrawCommand::new(pipe_moon, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_props.bind_group, Vec::new()).with_bind_group(1, tex_noise.bind_group, Vec::new()).with_bind_group(2, moon_uni, Vec::new()),
            DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_props.bind_group, Vec::new()).with_bind_group(1, cloud1, Vec::new()),
            DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_props.bind_group, Vec::new()).with_bind_group(1, cloud2, Vec::new()),
            DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_props.bind_group, Vec::new()).with_bind_group(1, cloud3, Vec::new()),
            DrawCommand::new(pipe_cloud, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, tex_props.bind_group, Vec::new()).with_bind_group(1, cloud4, Vec::new()),
        ]);
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen { color: final_target_id, width: 800, height: 600 })
            .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        final_graph.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_post, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, scene_view, Vec::new()).with_bind_group(1, post_uni_bg, Vec::new()),
        ]);

        let execute_pair = |h: &mut DesktopTestHarness<'_>, scene: &RenderGraph, final_graph: &RenderGraph| {
            let scene_submission = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, scene).expect("TC08.5 scene execution failed");
            let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(scene_submission), timeout: None });
            let final_submission = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, final_graph).expect("TC08.5 final execution failed");
            let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(final_submission), timeout: None });
        };
        let cold_start = Instant::now();
        execute_pair(&mut h, &scene_graph, &final_graph);
        let cold_render_time = cold_start.elapsed();
        let warm_start = Instant::now();
        execute_pair(&mut h, &scene_graph, &final_graph);
        let warm_render_time = warm_start.elapsed();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        h.save_texture_to_file_checked(&final_target_tex, format, "tests/outputs/desktop/tc08_5_nightsky.png").expect("Failed to save TC08.5 output");
        let raw = h.engine.read_texture_to_raw_with_format_checked(&final_target_tex, format).expect("Failed to read TC08.5 raw output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc08_5_nightsky_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC08.5",
            "manifest": "tests/shared_assets/manifests/tc08_5_nightsky.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của 2 pass scene → final + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "pass_count": 2,
            "node_count": 2,
            "draw_commands": 8,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_render_time.as_secs_f64() * 1000.0
        });
        fs::write("tests/outputs/desktop/tc08_5_nightsky_desktop.json", serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    });
}
