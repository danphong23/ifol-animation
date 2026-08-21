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
struct TransitionUniform {
    progress: f32,
    effect_type: f32,
    direction_x: f32,
    direction_y: f32,
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
        .expect("TC18 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC18 value must be numeric") as f32)
        .collect()
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC18 value must have four elements")
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC18 operation: {id}"))
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
        .expect("TC18 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

fn execute_all(
    h: &mut DesktopTestHarness,
    scene_a: &RenderGraph,
    scene_b: &RenderGraph,
    transition: &RenderGraph,
) -> f64 {
    execute(h, scene_a) + execute(h, scene_b) + execute(h, transition)
}

#[test]
fn run_tc18_transition() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc18_transition.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC18 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let passes = graph_spec["passes"].as_array().unwrap();
        let mut h = DesktopTestHarness::new(width, height).await;

        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let sky_pipeline = h.register_sky_pipeline();
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let transition_pipeline = h.register_transition_pipeline();

        let sky_a_spec = operation(operations, "sky_a");
        let sky_a_top = values(&sky_a_spec["uniform"]["top_color"]);
        let sky_a_bottom = values(&sky_a_spec["uniform"]["bottom_color"]);
        let sky_a_uniform = h.create_custom_uniform_bind_group(
            SkyUniform {
                top_color: [sky_a_top[0], sky_a_top[1], sky_a_top[2]],
                noise_strength: sky_a_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom_color: [sky_a_bottom[0], sky_a_bottom[1], sky_a_bottom[2]],
                time: sky_a_spec["uniform"]["time"].as_f64().unwrap() as f32,
            },
            "TC18 Sky A Uniform",
        );
        let sky_b_spec = operation(operations, "sky_b");
        let sky_b_top = values(&sky_b_spec["uniform"]["top_color"]);
        let sky_b_bottom = values(&sky_b_spec["uniform"]["bottom_color"]);
        let sky_b_uniform = h.create_custom_uniform_bind_group(
            SkyUniform {
                top_color: [sky_b_top[0], sky_b_top[1], sky_b_top[2]],
                noise_strength: sky_b_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom_color: [sky_b_bottom[0], sky_b_bottom[1], sky_b_bottom[2]],
                time: sky_b_spec["uniform"]["time"].as_f64().unwrap() as f32,
            },
            "TC18 Sky B Uniform",
        );
        let paladin_uniform = h.create_custom_uniform_bind_group(
            sprite_uniform(&h, &heroes, operation(operations, "paladin_a")),
            "TC18 Paladin A Uniform",
        );
        let mage_uniform = h.create_custom_uniform_bind_group(
            sprite_uniform(&h, &heroes, operation(operations, "mage_b")),
            "TC18 Mage B Uniform",
        );
        let transition_spec = operation(operations, "glitch_transition");
        let transition_uniform = h.create_custom_uniform_bind_group(
            TransitionUniform {
                progress: transition_spec["uniform"]["progress"].as_f64().unwrap() as f32,
                effect_type: transition_spec["uniform"]["effect_type"].as_f64().unwrap() as f32,
                direction_x: transition_spec["uniform"]["direction_x"].as_f64().unwrap() as f32,
                direction_y: transition_spec["uniform"]["direction_y"].as_f64().unwrap() as f32,
            },
            "TC18 Transition Uniform",
        );

        let (scene_a_id, _) = h.create_target("TC18 Scene A");
        let (scene_b_id, _) = h.create_target("TC18 Scene B");
        let (final_id, final_texture) = h.create_target("TC18 Final");
        let dual_texture =
            h.create_dual_texture_bind_group(scene_a_id, scene_b_id, "TC18 Dual Scene Textures");

        let mut scene_a_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_a_id,
            width,
            height,
        })
        .with_clear_color(value4(&passes[0]["clear_color"]));
        scene_a_graph.add_batch(
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
                .with_bind_group(1, sky_a_uniform, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, paladin_uniform, Vec::new()),
            ],
        );

        let mut scene_b_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_b_id,
            width,
            height,
        })
        .with_clear_color(value4(&passes[1]["clear_color"]));
        scene_b_graph.add_batch(
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
                .with_bind_group(1, sky_b_uniform, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, mage_uniform, Vec::new()),
            ],
        );

        let mut transition_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color(value4(&passes[2]["clear_color"]));
        transition_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                transition_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, dual_texture, Vec::new())
            .with_bind_group(1, transition_uniform, Vec::new())],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms =
            execute_all(&mut h, &scene_a_graph, &scene_b_graph, &transition_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC18 cold readback failed");
        let warm_render_time_ms =
            execute_all(&mut h, &scene_a_graph, &scene_b_graph, &transition_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC18 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC18 output changed between runs"
        );

        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc18_transition.png",
        )
        .expect("TC18 PNG save failed");
        fs::write(
            "tests/outputs/desktop/tc18_transition_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC18",
            "manifest": "tests/shared_assets/manifests/tc18_transition.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "3 pass (scene A → scene B → dual-texture transition) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": passes.len(),
            "instance_count": operations.iter().map(|item| item["instance_count"].as_u64().unwrap()).sum::<u64>(),
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc18_transition_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
