mod harness;

use harness::{DesktopTestHarness, LoadedTextureInfo, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

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

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn values(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("TC17 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC17 value must be numeric") as f32)
        .collect()
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC17 value must have four elements")
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC17 operation: {id}"))
}

fn sprite_uniform(
    h: &DesktopTestHarness,
    texture: &LoadedTextureInfo,
    spec: &Value,
) -> SpriteUniform {
    let crop = values(&spec["crop_uv"]);
    let position = values(&spec["position"]);
    let key_color = values(&spec["key_color"]);
    let crop_width = (crop[2] - crop[0]) * texture.width as f32;
    let crop_height = (crop[3] - crop[1]) * texture.height as f32;
    let crop_aspect = crop_width / crop_height.max(1.0);
    let height = spec["target_height_scale"].as_f64().unwrap() as f32;
    SpriteUniform {
        pos: [position[0], position[1]],
        scale: [
            height * (crop_aspect / (h.width as f32 / h.height as f32)),
            height,
        ],
        uv_min: [crop[0], crop[1]],
        uv_max: [crop[2], crop[3]],
        key_color: [key_color[0], key_color[1], key_color[2]],
        tolerance: spec["tolerance"].as_f64().unwrap() as f32,
        smoothness: spec["smoothness"].as_f64().unwrap() as f32,
        z_depth: spec["z_depth"].as_f64().unwrap() as f32,
        opacity: spec["opacity"].as_f64().unwrap() as f32,
        _pad: 0.0,
    }
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC17 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

fn execute_all(
    h: &mut DesktopTestHarness,
    heroes_graph: &RenderGraph,
    final_graph: &RenderGraph,
) -> f64 {
    execute(h, heroes_graph) + execute(h, final_graph)
}

#[test]
fn run_tc17_outline() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc17_outline.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC17 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let passes = graph_spec["passes"].as_array().unwrap();
        let mut h = DesktopTestHarness::new(width, height).await;

        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let items = h.load_texture_exact("canonical_sprites_items.png");
        let sky_pipeline = h.register_sky_pipeline();
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let outline_pipeline = h.register_pipeline(
            "outline_shadow.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let sky_spec = operation(operations, "sky");
        let sky_top = values(&sky_spec["uniform"]["top_color"]);
        let sky_bottom = values(&sky_spec["uniform"]["bottom_color"]);
        let sky_uniform = h.create_custom_uniform_bind_group(
            SkyUniform {
                top_color: [sky_top[0], sky_top[1], sky_top[2]],
                noise_strength: sky_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom_color: [sky_bottom[0], sky_bottom[1], sky_bottom[2]],
                time: sky_spec["uniform"]["time"].as_f64().unwrap() as f32,
            },
            "TC17 Sky Uniform",
        );
        let paladin_uniform = h.create_custom_uniform_bind_group(
            sprite_uniform(&h, &heroes, operation(operations, "paladin")),
            "TC17 Paladin Uniform",
        );
        let mage_uniform = h.create_custom_uniform_bind_group(
            sprite_uniform(&h, &heroes, operation(operations, "mage")),
            "TC17 Mage Uniform",
        );
        let chest_uniform = h.create_custom_uniform_bind_group(
            sprite_uniform(&h, &items, operation(operations, "chest")),
            "TC17 Chest Uniform",
        );
        let outline_spec = operation(operations, "outline_shadow");
        let outline_uniform = h.create_custom_uniform_bind_group(
            OutlineUniform {
                outline_color: value4(&outline_spec["uniform"]["outline_color"]),
                shadow_color: value4(&outline_spec["uniform"]["shadow_color"]),
                shadow_offset: values(&outline_spec["uniform"]["shadow_offset"])
                    .try_into()
                    .unwrap(),
                texel_size: values(&outline_spec["uniform"]["texel_size"])
                    .try_into()
                    .unwrap(),
                outline_thickness: outline_spec["uniform"]["outline_thickness"]
                    .as_f64()
                    .unwrap() as f32,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            },
            "TC17 Outline Uniform",
        );

        let (heroes_target_id, _) = h.create_target("TC17 Transparent Heroes");
        let heroes_view = h.create_texture_bind_group(heroes_target_id, "TC17 Heroes View");
        let (final_target_id, final_texture) = h.create_target("TC17 Final Target");

        let heroes_clear = value4(&passes[0]["clear_color"]);
        let final_clear = value4(&passes[1]["clear_color"]);
        let mut heroes_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: heroes_target_id,
            width,
            height,
        })
        .with_clear_color(heroes_clear);
        heroes_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, paladin_uniform, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, mage_uniform, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, items.bind_group, Vec::new())
                .with_bind_group(1, chest_uniform, Vec::new()),
            ],
        );

        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width,
            height,
        })
        .with_clear_color(final_clear);
        final_graph.add_batch(
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
                .with_bind_group(1, sky_uniform, Vec::new()),
                DrawCommand::new(
                    outline_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes_view, Vec::new())
                .with_bind_group(1, outline_uniform, Vec::new()),
            ],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute_all(&mut h, &heroes_graph, &final_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC17 cold readback failed");
        let warm_render_time_ms = execute_all(&mut h, &heroes_graph, &final_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC17 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC17 output changed between runs"
        );

        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc17_outline.png",
        )
        .expect("TC17 PNG save failed");
        fs::write("tests/outputs/desktop/tc17_outline_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC17",
            "manifest": "tests/shared_assets/manifests/tc17_outline.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "2 pass (transparent heroes → sky/outline final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": passes.len(),
            "instance_count": operations.iter().map(|operation| operation["instance_count"].as_u64().unwrap()).sum::<u64>(),
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc17_outline_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
