mod harness;

use harness::{DesktopTestHarness, LoadedTextureInfo, SpriteUniform};
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
struct ParticleSimUniform {
    time: f32,
    wind_speed: f32,
    gravity: f32,
    particle_count: f32,
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn array_f32(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("TC15 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC15 value must be numeric") as f32)
        .collect()
}

fn fixed<const N: usize>(value: &Value) -> [f32; N] {
    array_f32(value)
        .try_into()
        .unwrap_or_else(|_| panic!("TC15 value must contain {N} numbers"))
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC15 operation: {id}"))
}

fn sprite_uniform(
    h: &DesktopTestHarness,
    texture: &LoadedTextureInfo,
    spec: &Value,
) -> SpriteUniform {
    let crop = fixed::<4>(&spec["crop_uv"]);
    let position = fixed::<2>(&spec["position"]);
    let key_color = fixed::<3>(&spec["key_color"]);
    let crop_aspect = ((crop[2] - crop[0]) * texture.width as f32)
        / ((crop[3] - crop[1]) * texture.height as f32).max(1.0);
    let height = spec["target_height_scale"].as_f64().unwrap() as f32;
    SpriteUniform {
        pos: position,
        scale: [
            height * (crop_aspect / (h.width as f32 / h.height as f32)),
            height,
        ],
        uv_min: [crop[0], crop[1]],
        uv_max: [crop[2], crop[3]],
        key_color,
        tolerance: spec["tolerance"].as_f64().unwrap() as f32,
        smoothness: spec["smoothness"].as_f64().unwrap() as f32,
        z_depth: spec["z_depth"].as_f64().unwrap() as f32,
        opacity: spec["opacity"].as_f64().unwrap() as f32,
        _pad: 0.0,
    }
}

fn execute_pass(h: &mut DesktopTestHarness, graph: &RenderGraph) {
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC15 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}

#[test]
fn run_tc15_snow() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc15_snow.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC15 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let mut h = DesktopTestHarness::new(width, height).await;

        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let props = h.load_texture_exact("canonical_tc085_props.png");
        let forest = h.load_texture_exact("canonical_bg_forest_props1.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let snow = h.load_texture_exact("canonical_particle_snow.png");

        let sky_pipeline = h.register_sky_pipeline();
        let moon_pipeline = h.register_moon_pipeline();
        let cloud_pipeline = h.register_pipeline(
            "cloud_depth.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let snow_pipeline = h.register_pipeline(
            "snow_physics_instanced.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let sky_spec = operation(operations, "winter_sky");
        let sky_top = fixed::<3>(&sky_spec["uniform"]["top_color"]);
        let sky_bottom = fixed::<3>(&sky_spec["uniform"]["bottom_color"]);
        let sky_bg = h.create_custom_uniform_bind_group(
            SkyUniform {
                top_color: sky_top,
                noise_strength: sky_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom_color: sky_bottom,
                time: sky_spec["uniform"]["time"].as_f64().unwrap() as f32,
            },
            "TC15 Winter Sky Uniform",
        );

        let moon_spec = operation(operations, "winter_moon");
        let moon_bg = h.create_custom_uniform_bind_group(
            MoonUniform {
                model_view: fixed::<16>(&moon_spec["uniform"]["model_view"]),
                uv_min: fixed::<2>(&moon_spec["uniform"]["uv_min"]),
                uv_max: fixed::<2>(&moon_spec["uniform"]["uv_max"]),
                key_color: fixed::<3>(&moon_spec["uniform"]["key_color"]),
                tolerance: moon_spec["uniform"]["tolerance"].as_f64().unwrap() as f32,
                smoothness: moon_spec["uniform"]["smoothness"].as_f64().unwrap() as f32,
                noise_strength: moon_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                glow_intensity: moon_spec["uniform"]["glow_intensity"].as_f64().unwrap() as f32,
                _pad: moon_spec["uniform"]["_pad"].as_f64().unwrap() as f32,
            },
            "TC15 Moon Uniform",
        );

        let cloud_spec = operation(operations, "winter_cloud");
        let cloud_bg = h.create_custom_uniform_bind_group(
            CloudUniform {
                model_view: fixed::<16>(&cloud_spec["uniform"]["model_view"]),
                uv_bounds: fixed::<4>(&cloud_spec["uniform"]["uv_bounds"]),
                key_color_tol: fixed::<4>(&cloud_spec["uniform"]["key_color_tol"]),
                params: fixed::<4>(&cloud_spec["uniform"]["params"]),
                lighting_pos: fixed::<4>(&cloud_spec["uniform"]["lighting_pos"]),
            },
            "TC15 Cloud Uniform",
        );

        let pine_left_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "pine_left"));
            h.create_custom_uniform_bind_group(uniform, "TC15 Pine Left Uniform")
        };
        let pine_right_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "pine_right"));
            h.create_custom_uniform_bind_group(uniform, "TC15 Pine Right Uniform")
        };
        let paladin_bg = {
            let uniform = sprite_uniform(&h, &heroes, operation(operations, "paladin"));
            h.create_custom_uniform_bind_group(uniform, "TC15 Paladin Uniform")
        };
        let snow_spec = operation(operations, "snow_particles");
        let snow_bg = h.create_custom_uniform_bind_group(
            ParticleSimUniform {
                time: snow_spec["uniform"]["time"].as_f64().unwrap() as f32,
                wind_speed: snow_spec["uniform"]["wind_speed"].as_f64().unwrap() as f32,
                gravity: snow_spec["uniform"]["gravity"].as_f64().unwrap() as f32,
                particle_count: snow_spec["uniform"]["particle_count"].as_f64().unwrap() as f32,
            },
            "TC15 Snow Simulation Uniform",
        );

        let (target_id, target_texture) = h.create_target("TC15 Snow Target");
        let clear = fixed::<4>(&graph_spec["passes"][0]["clear_color"]);
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(clear);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sky_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, noise.bind_group, Vec::new())
                .with_bind_group(1, sky_bg, Vec::new()),
                DrawCommand::new(
                    moon_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, props.bind_group, Vec::new())
                .with_bind_group(1, noise.bind_group, Vec::new())
                .with_bind_group(2, moon_bg, Vec::new()),
                DrawCommand::new(
                    cloud_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, props.bind_group, Vec::new())
                .with_bind_group(1, cloud_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, forest.bind_group, Vec::new())
                .with_bind_group(1, pine_left_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, forest.bind_group, Vec::new())
                .with_bind_group(1, pine_right_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, paladin_bg, Vec::new()),
                DrawCommand::new(
                    snow_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..200,
                    },
                )
                .with_bind_group(0, snow.bind_group, Vec::new())
                .with_bind_group(1, snow_bg, Vec::new()),
            ],
        );

        let execute = |h: &mut DesktopTestHarness| {
            let started = Instant::now();
            execute_pass(h, &graph);
            started.elapsed().as_secs_f64() * 1000.0
        };
        let cold_render_time_ms = execute(&mut h);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC15 cold readback failed");
        let warm_render_time_ms = execute(&mut h);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC15 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC15 output changed between cold and warm runs"
        );

        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc15_snow.png",
        )
        .expect("TC15 PNG save failed");
        fs::write("tests/outputs/desktop/tc15_snow_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC15",
            "manifest": "tests/shared_assets/manifests/tc15_snow.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (winter snow scene) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": graph_spec["passes"].as_array().unwrap().len(),
            "instance_count": snow_spec["instance_count"],
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc15_snow_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
