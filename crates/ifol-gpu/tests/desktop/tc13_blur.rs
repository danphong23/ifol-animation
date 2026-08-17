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
struct BlurUniform {
    direction: [f32; 2],
    radius: f32,
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

fn array_f32(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("TC13 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC13 value must be numeric") as f32)
        .collect()
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC13 operation: {id}"))
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

fn execute_pass(h: &mut DesktopTestHarness, graph: &RenderGraph) {
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC13 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}

fn execute_all(
    h: &mut DesktopTestHarness,
    background: &RenderGraph,
    blur_horizontal: &RenderGraph,
    blur_vertical: &RenderGraph,
    final_composite: &RenderGraph,
) -> f64 {
    let started = Instant::now();
    execute_pass(h, background);
    execute_pass(h, blur_horizontal);
    execute_pass(h, blur_vertical);
    execute_pass(h, final_composite);
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc13_blur() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc13_blur.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC13 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
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
        let blur_pipeline = h.register_pipeline(
            "gaussian_blur_separable.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );
        let blit_pipeline = h.register_pipeline(
            "texture_blit.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            false,
        );
        let wisps_pipeline = h.register_pipeline(
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

        let sky_spec = operation(operations, "forest_sky");
        let top = array_f32(&sky_spec["uniform"]["top_color"]);
        let bottom = array_f32(&sky_spec["uniform"]["bottom_color"]);
        let sky_bg = h.create_custom_uniform_bind_group(
            SkyUniform {
                top_color: [top[0], top[1], top[2]],
                noise_strength: sky_spec["uniform"]["noise_strength"].as_f64().unwrap() as f32,
                bottom_color: [bottom[0], bottom[1], bottom[2]],
                time: sky_spec["uniform"]["time"].as_f64().unwrap() as f32,
            },
            "TC13 Forest Sky Uniform",
        );
        let tree_left_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "tree_left"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Tree Left Uniform")
        };
        let tree_center_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "tree_center"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Tree Center Uniform")
        };
        let tree_right_bg = {
            let uniform = sprite_uniform(&h, &forest, operation(operations, "tree_right"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Tree Right Uniform")
        };
        let paladin_bg = {
            let uniform = sprite_uniform(&h, &heroes, operation(operations, "paladin_foreground"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Paladin Uniform")
        };
        let archer_bg = {
            let uniform = sprite_uniform(&h, &heroes, operation(operations, "archer_foreground"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Archer Uniform")
        };
        let chest_bg = {
            let uniform = sprite_uniform(&h, &items, operation(operations, "chest_foreground"));
            h.create_custom_uniform_bind_group(uniform, "TC13 Chest Uniform")
        };
        let blur_h_spec = operation(operations, "blur_horizontal");
        let blur_h_direction = array_f32(&blur_h_spec["uniform"]["direction"]);
        let blur_h_bg = h.create_custom_uniform_bind_group(
            BlurUniform {
                direction: [blur_h_direction[0], blur_h_direction[1]],
                radius: blur_h_spec["uniform"]["radius"].as_f64().unwrap() as f32,
                _pad: 0.0,
            },
            "TC13 Horizontal Blur Uniform",
        );
        let blur_v_spec = operation(operations, "blur_vertical");
        let blur_v_direction = array_f32(&blur_v_spec["uniform"]["direction"]);
        let blur_v_bg = h.create_custom_uniform_bind_group(
            BlurUniform {
                direction: [blur_v_direction[0], blur_v_direction[1]],
                radius: blur_v_spec["uniform"]["radius"].as_f64().unwrap() as f32,
                _pad: 0.0,
            },
            "TC13 Vertical Blur Uniform",
        );

        let (background_a_id, _) = h.create_target("TC13 Background A");
        let background_a_view = h.create_texture_bind_group(background_a_id, "TC13 Background A View");
        let (blur_b_id, _) = h.create_target("TC13 Blur B");
        let blur_b_view = h.create_texture_bind_group(blur_b_id, "TC13 Blur B View");
        let (final_id, final_texture) = h.create_target("TC13 Final Composite");

        let clear_background = [0.02, 0.10, 0.15, 1.0];
        let mut background_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: background_a_id,
            width,
            height,
        })
        .with_clear_color(clear_background);
        background_graph.add_batch(
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
                    wisps_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..40,
                    },
                )
                .with_bind_group(0, props.bind_group, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, forest.bind_group, Vec::new())
                .with_bind_group(1, tree_left_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, forest.bind_group, Vec::new())
                .with_bind_group(1, tree_center_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, forest.bind_group, Vec::new())
                .with_bind_group(1, tree_right_bg, Vec::new()),
            ],
        );
        let mut blur_horizontal_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: blur_b_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        blur_horizontal_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                blur_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, background_a_view, Vec::new())
            .with_bind_group(1, blur_h_bg, Vec::new())],
        );
        let mut blur_vertical_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: background_a_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        blur_vertical_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                blur_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, blur_b_view, Vec::new())
            .with_bind_group(1, blur_v_bg, Vec::new())],
        );
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        final_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    blit_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, background_a_view, Vec::new()),
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
                .with_bind_group(1, archer_bg, Vec::new()),
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, items.bind_group, Vec::new())
                .with_bind_group(1, chest_bg, Vec::new()),
            ],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute_all(
            &mut h,
            &background_graph,
            &blur_horizontal_graph,
            &blur_vertical_graph,
            &final_graph,
        );
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC13 cold readback failed");
        let warm_render_time_ms = execute_all(
            &mut h,
            &background_graph,
            &blur_horizontal_graph,
            &blur_vertical_graph,
            &final_graph,
        );
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("TC13 warm readback failed");
        assert_eq!(cold_raw.bytes, raw.bytes, "TC13 output changed between cold and warm runs");

        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc13_blur.png",
        )
        .expect("TC13 PNG save failed");
        fs::write("tests/outputs/desktop/tc13_blur_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC13",
            "manifest": "tests/shared_assets/manifests/tc13_blur.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "4 pass (background → blur H → blur V → final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": graph_spec["passes"].as_array().unwrap().len(),
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc13_blur_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
