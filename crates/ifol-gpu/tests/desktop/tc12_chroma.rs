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
        .expect("TC12 uniform value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC12 uniform item must be numeric") as f32)
        .collect()
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC12 operation: {id}"))
}

fn sprite_uniform(
    h: &DesktopTestHarness,
    texture: &LoadedTextureInfo,
    spec: &Value,
) -> SpriteUniform {
    let crop = array_f32(&spec["crop_uv"]);
    let position = array_f32(&spec["position"]);
    let key_color = array_f32(&spec["key_color"]);
    let crop_width = (crop[2] - crop[0]) * texture.width as f32;
    let crop_height = (crop[3] - crop[1]) * texture.height as f32;
    let crop_aspect = crop_width / crop_height.max(1.0);
    let screen_aspect = h.width as f32 / h.height as f32;
    let target_height_scale = spec["target_height_scale"].as_f64().unwrap() as f32;

    SpriteUniform {
        pos: [position[0], position[1]],
        scale: [target_height_scale * (crop_aspect / screen_aspect), target_height_scale],
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

fn execute_graph(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC12 graph execution failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc12_chroma() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc12_chroma.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC12 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
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

        let sky_uniform_values = array_f32(&operation(operations, "sky")["uniform"]["top_color"]);
        let sky_spec = operation(operations, "sky");
        let bottom_color = array_f32(&sky_spec["uniform"]["bottom_color"]);
        let sky_uniform = SkyUniform {
            top_color: [sky_uniform_values[0], sky_uniform_values[1], sky_uniform_values[2]],
            noise_strength: sky_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
            bottom_color: [bottom_color[0], bottom_color[1], bottom_color[2]],
            time: sky_spec["uniform"]["time"].as_f64().unwrap() as f32,
        };
        let sky_bg = h.create_custom_uniform_bind_group(sky_uniform, "TC12 Sky Uniform");

        let paladin = operation(operations, "paladin");
        let mage = operation(operations, "mage");
        let scroll = operation(operations, "scroll");
        let potion = operation(operations, "potion");
        let bag = operation(operations, "bag");
        let paladin_bg = h.create_sprite_uniform_bind_group(sprite_uniform(&h, &heroes, paladin));
        let mage_bg = h.create_sprite_uniform_bind_group(sprite_uniform(&h, &heroes, mage));
        let scroll_bg = h.create_sprite_uniform_bind_group(sprite_uniform(&h, &items, scroll));
        let potion_bg = h.create_sprite_uniform_bind_group(sprite_uniform(&h, &items, potion));
        let bag_bg = h.create_sprite_uniform_bind_group(sprite_uniform(&h, &items, bag));

        let (target_id, target_texture) = h.create_target("TC12 Target");
        let clear = array_f32(&graph_spec["clear_color"]);
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color([clear[0], clear[1], clear[2], clear[3]]);
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
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, paladin_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, mage_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, items.bind_group, Vec::new())
                .with_bind_group(1, scroll_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, items.bind_group, Vec::new())
                .with_bind_group(1, potion_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, items.bind_group, Vec::new())
                .with_bind_group(1, bag_bg, Vec::new()),
            ],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute_graph(&mut h, &graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC12 cold readback failed");
        let warm_render_time_ms = execute_graph(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC12 warm readback failed");
        assert_eq!(cold_raw.bytes, raw.bytes, "TC12 output changed between cold and warm runs");

        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc12_chroma.png",
        )
        .expect("TC12 PNG save failed");
        fs::write("tests/outputs/desktop/tc12_chroma_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC12",
            "manifest": "tests/shared_assets/manifests/tc12_chroma.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "6 draw command execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": 1,
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc12_chroma_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
