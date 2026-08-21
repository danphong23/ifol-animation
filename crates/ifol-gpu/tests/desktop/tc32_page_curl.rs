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
struct PageCurlUniform {
    progress: f32,
    radius: f32,
    _pad: [f32; 2],
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn sprite_uniform(
    width: u32,
    height: u32,
    tex_width: u32,
    tex_height: u32,
    value: &Value,
) -> harness::SpriteUniform {
    let uv_min = &value["uv_min"];
    let uv_max = &value["uv_max"];
    let crop_width = (uv_max[0].as_f64().unwrap() - uv_min[0].as_f64().unwrap()) * tex_width as f64;
    let crop_height =
        (uv_max[1].as_f64().unwrap() - uv_min[1].as_f64().unwrap()) * tex_height as f64;
    let scale_y = value["scale_y"].as_f64().unwrap();
    let scale_x = scale_y * crop_width / crop_height / (width as f64 / height as f64);
    harness::SpriteUniform {
        pos: [0.0, 0.0],
        scale: [scale_x as f32, scale_y as f32],
        uv_min: [
            uv_min[0].as_f64().unwrap() as f32,
            uv_min[1].as_f64().unwrap() as f32,
        ],
        uv_max: [
            uv_max[0].as_f64().unwrap() as f32,
            uv_max[1].as_f64().unwrap() as f32,
        ],
        key_color: [0.0, 1.0, 0.0],
        tolerance: value["tolerance"].as_f64().unwrap() as f32,
        smoothness: value["smoothness"].as_f64().unwrap() as f32,
        z_depth: 0.5,
        opacity: 1.0,
        _pad: 0.0,
    }
}

fn sky_uniform(value: &Value) -> SkyUniform {
    SkyUniform {
        top_color: [
            value["top_color"][0].as_f64().unwrap() as f32,
            value["top_color"][1].as_f64().unwrap() as f32,
            value["top_color"][2].as_f64().unwrap() as f32,
        ],
        noise_strength: value["noise_strength"].as_f64().unwrap() as f32,
        bottom_color: [
            value["bottom_color"][0].as_f64().unwrap() as f32,
            value["bottom_color"][1].as_f64().unwrap() as f32,
            value["bottom_color"][2].as_f64().unwrap() as f32,
        ],
        time: value["time"].as_f64().unwrap() as f32,
    }
}

fn execute_all(h: &mut DesktopTestHarness, graphs: &[&RenderGraph]) -> f64 {
    let started = Instant::now();
    let mut submission = None;
    for graph in graphs {
        submission = Some(
            h.executor
                .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
                .expect("TC32 graph pass failed"),
        );
    }
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: submission,
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc32_page_curl() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc32_page_curl.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let graph = &manifest["graph"];
        let operations = graph["operations"].as_array().unwrap();
        let width = graph["target"]["width"].as_u64().unwrap() as u32;
        let height = graph["target"]["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let sky_pipeline = h.register_pipeline(
            "sky_composite_deterministic.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let curl_pipeline = h.register_dual_texture_pipeline(
            "page_curl.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );
        let sky_a_bg = h
            .create_custom_uniform_bind_group(sky_uniform(&operations[0]["uniform"]), "TC32 Sky A");
        let sky_b_bg = h
            .create_custom_uniform_bind_group(sky_uniform(&operations[2]["uniform"]), "TC32 Sky B");
        let paladin_bg = h.create_custom_uniform_bind_group(
            sprite_uniform(
                width,
                height,
                heroes.width,
                heroes.height,
                &operations[1]["uniform"],
            ),
            "TC32 Paladin",
        );
        let mage_bg = h.create_custom_uniform_bind_group(
            sprite_uniform(
                width,
                height,
                heroes.width,
                heroes.height,
                &operations[3]["uniform"],
            ),
            "TC32 Mage",
        );
        let curl = &operations[4]["uniform"];
        let curl_bg = h.create_custom_uniform_bind_group(
            PageCurlUniform {
                progress: curl["progress"].as_f64().unwrap() as f32,
                radius: curl["radius"].as_f64().unwrap() as f32,
                _pad: [0.0, 0.0],
            },
            "TC32 Page Curl",
        );
        let (scene_a_id, _scene_a_texture) = h.create_target("TC32 Scene A");
        let (scene_b_id, _scene_b_texture) = h.create_target("TC32 Scene B");
        let (final_id, final_texture) = h.create_target("TC32 Final");
        let curl_textures =
            h.create_dual_texture_bind_group(scene_a_id, scene_b_id, "TC32 Dual Scenes");

        let mut graph_a = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_a_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_a.add_batch(
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
                .with_bind_group(1, sky_a_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, paladin_bg, Vec::new()),
            ],
        );
        let mut graph_b = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_b_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_b.add_batch(
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
                .with_bind_group(1, sky_b_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, mage_bg, Vec::new()),
            ],
        );
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph_final.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                curl_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, curl_textures, Vec::new())
            .with_bind_group(1, curl_bg, Vec::new())],
        );

        let graphs = [&graph_a, &graph_b, &graph_final];
        let cold_ms = execute_all(&mut h, &graphs);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC32 cold readback failed");
        let warm_ms = execute_all(&mut h, &graphs);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC32 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC32 output changed between runs"
        );
        let output_path = std::path::Path::new("tests/outputs/desktop/tc32_page_curl.png");
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            output_path,
        )
        .expect("TC32 PNG export failed");
        fs::write(
            "tests/outputs/desktop/tc32_page_curl_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC32", "width": raw.width, "height": raw.height, "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name, "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "3 pass (scene A + scene B + page curl) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "raw_fingerprint": fnv1a64(&raw.bytes), "manifest": "tests/shared_assets/manifests/tc32_page_curl.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()), "cold_render_time_ms": cold_ms,
            "warm_render_time_ms": warm_ms, "warm_iteration_count": 1, "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
            "cache_output_equal": true, "node_count": graph["node_count"], "draw_commands": graph["command_count"],
            "instance_count": 5, "pass_count": graph["passes"].as_array().unwrap().len()
        });
        fs::write(
            "tests/outputs/desktop/tc32_page_curl_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
