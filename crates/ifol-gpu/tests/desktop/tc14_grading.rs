mod harness;

use harness::{DesktopTestHarness, LoadedTextureInfo, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ColorGradingUniform {
    params: [f32; 4],
    shadow_tint_vig: [f32; 4],
    highlight_tint: [f32; 4],
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
        .expect("TC14 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC14 value must be numeric") as f32)
        .collect()
}

fn color4(value: &Value) -> [f32; 4] {
    let values = array_f32(value);
    values.try_into().expect("TC14 color must have four values")
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC14 operation: {id}"))
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
    let target_height_scale = spec["target_height_scale"].as_f64().unwrap() as f32;
    SpriteUniform {
        pos: [position[0], position[1]],
        scale: [
            target_height_scale * (crop_aspect / (h.width as f32 / h.height as f32)),
            target_height_scale,
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

fn execute_pass(h: &mut DesktopTestHarness, graph: &RenderGraph) {
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC14 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}

fn execute_all(h: &mut DesktopTestHarness, scene: &RenderGraph, grading: &RenderGraph) -> f64 {
    let started = Instant::now();
    execute_pass(h, scene);
    execute_pass(h, grading);
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc14_grading() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc14_grading.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC14 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let passes = graph_spec["passes"].as_array().unwrap();
        let mut h = DesktopTestHarness::new(width, height).await;

        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let forest = h.load_texture_exact("canonical_bg_forest_props1.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let items = h.load_texture_exact("canonical_sprites_items.png");
        let props = h.load_texture_exact("canonical_tc085_props.png");

        let sky_pipeline = h.register_sky_pipeline();
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let sparks_pipeline = h.register_pipeline(
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
        let grading_pipeline = h.register_pipeline(
            "color_grading_filmic.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );

        let sky_spec = operation(operations, "sunset_sky");
        let top = array_f32(&sky_spec["uniform"]["top_color"]);
        let bottom = array_f32(&sky_spec["uniform"]["bottom_color"]);
        let sky_bg = h.create_custom_uniform_bind_group(
            [
                top[0], top[1], top[2],
                sky_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom[0], bottom[1], bottom[2],
                sky_spec["uniform"]["time"].as_f64().unwrap() as f32,
            ],
            "TC14 Sunset Sky Uniform",
        );
        let tree_left_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "tree_left"));
            h.create_custom_uniform_bind_group(uniform, "TC14 Tree Left Uniform")
        };
        let tree_right_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "tree_right"));
            h.create_custom_uniform_bind_group(uniform, "TC14 Tree Right Uniform")
        };
        let paladin_bg = {
            let uniform = sprite_uniform(&h, &heroes, operation(operations, "paladin"));
            h.create_custom_uniform_bind_group(uniform, "TC14 Paladin Uniform")
        };
        let mage_bg = {
            let uniform = sprite_uniform(&h, &heroes, operation(operations, "mage"));
            h.create_custom_uniform_bind_group(uniform, "TC14 Mage Uniform")
        };
        let chest_bg = {
            let uniform = sprite_uniform(&h, &items, operation(operations, "chest"));
            h.create_custom_uniform_bind_group(uniform, "TC14 Chest Uniform")
        };
        let grade_spec = operation(operations, "color_grade");
        let params: [f32; 4] = array_f32(&grade_spec["uniform"]["params"])
            .try_into()
            .unwrap();
        let shadow_tint_vig: [f32; 4] = array_f32(&grade_spec["uniform"]["shadow_tint_vig"])
            .try_into()
            .unwrap();
        let highlight_tint: [f32; 4] = array_f32(&grade_spec["uniform"]["highlight_tint"])
            .try_into()
            .unwrap();
        let grade_bg = h.create_custom_uniform_bind_group(
            ColorGradingUniform { params, shadow_tint_vig, highlight_tint },
            "TC14 Color Grading Uniform",
        );

        let (scene_id, _) = h.create_target("TC14 Scene Target");
        let scene_view = h.create_texture_bind_group(scene_id, "TC14 Scene View");
        let (final_id, final_texture) = h.create_target("TC14 Final Target");
        let scene_clear = color4(&passes[0]["clear_color"]);
        let final_clear = color4(&passes[1]["clear_color"]);

        let mut scene_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_id,
            width,
            height,
        })
        .with_clear_color(scene_clear);
        scene_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(sky_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, noise.bind_group, Vec::new())
                    .with_bind_group(1, sky_bg, Vec::new()),
                DrawCommand::new(sparks_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..40 })
                    .with_bind_group(0, props.bind_group, Vec::new()),
                DrawCommand::new(chroma_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, forest.bind_group, Vec::new())
                    .with_bind_group(1, tree_left_bg, Vec::new()),
                DrawCommand::new(chroma_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, forest.bind_group, Vec::new())
                    .with_bind_group(1, tree_right_bg, Vec::new()),
                DrawCommand::new(chroma_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, heroes.bind_group, Vec::new())
                    .with_bind_group(1, paladin_bg, Vec::new()),
                DrawCommand::new(chroma_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, heroes.bind_group, Vec::new())
                    .with_bind_group(1, mage_bg, Vec::new()),
                DrawCommand::new(chroma_pipeline, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, items.bind_group, Vec::new())
                    .with_bind_group(1, chest_bg, Vec::new()),
            ],
        );

        let mut grading_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color(final_clear);
        grading_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                grading_pipeline,
                DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 },
            )
            .with_bind_group(0, scene_view, Vec::new())
            .with_bind_group(1, grade_bg, Vec::new())],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute_all(&mut h, &scene_graph, &grading_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC14 cold readback failed");
        let warm_render_time_ms = execute_all(&mut h, &scene_graph, &grading_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC14 warm readback failed");
        assert_eq!(cold_raw.bytes, raw.bytes, "TC14 output changed between cold and warm runs");

        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc14_grading.png",
        )
        .expect("TC14 PNG save failed");
        fs::write("tests/outputs/desktop/tc14_grading_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC14",
            "manifest": "tests/shared_assets/manifests/tc14_grading.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "2 pass (scene → color grading) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": passes.len(),
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc14_grading_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
