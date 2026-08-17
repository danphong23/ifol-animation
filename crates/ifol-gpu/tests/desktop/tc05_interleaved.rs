mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn array2(value: &Value) -> [f32; 2] {
    [value[0].as_f64().unwrap() as f32, value[1].as_f64().unwrap() as f32]
}

fn execute_chain(h: &mut DesktopTestHarness<'_>, graphs: &[RenderGraph]) -> Duration {
    let started = Instant::now();
    let mut last_submission = None;
    for graph in graphs {
        last_submission = Some(
            h.executor
                .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
                .expect("TC05 pass execution failed"),
        );
    }
    let submission = last_submission.expect("TC05 graph chain must contain passes");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed()
}

#[test]
fn run_tc05_interleaved() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc05_interleaved.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC05 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;

        let mut pipelines = HashMap::new();
        for (pipeline_name, pipeline_spec) in graph_spec["pipelines"].as_object().unwrap() {
            let blend = match pipeline_spec["blend"].as_str().unwrap() {
                "Replace" => wgpu::BlendState::REPLACE,
                "AlphaBlend" => wgpu::BlendState::ALPHA_BLENDING,
                other => panic!("Unsupported TC05 blend mode: {other}"),
            };
            let shader = pipeline_spec["shader"].as_str().unwrap();
            let has_uniform = pipeline_spec["has_uniform"].as_bool().unwrap();
            pipelines.insert(
                pipeline_name.clone(),
                h.register_pipeline(shader, Some(blend), false, has_uniform),
            );
        }

        let mut targets = HashMap::new();
        let mut target_textures = HashMap::new();
        for target_spec in graph_spec["targets"].as_array().unwrap() {
            let target_id = target_spec["id"].as_str().unwrap().to_owned();
            let (handle, texture) = h.create_target(&format!("TC05 Target {target_id}"));
            targets.insert(target_id.clone(), handle);
            target_textures.insert(target_id, texture);
        }

        let mut target_bind_groups = HashMap::new();
        for target_id in ["A", "B"] {
            let target = *targets.get(target_id).expect("TC05 source target missing");
            let bind_group = h.create_texture_bind_group(target, &format!("TC05 Target {target_id} View"));
            target_bind_groups.insert(target_id.to_owned(), bind_group);
        }

        let mut graphs = Vec::new();
        for pass_spec in graph_spec["passes"].as_array().unwrap() {
            let target_id = pass_spec["target"].as_str().unwrap();
            let color = *targets.get(target_id).expect("TC05 pass target missing");
            let clear = &pass_spec["clear_color"];
            let clear_color = [
                clear[0].as_f64().unwrap() as f32,
                clear[1].as_f64().unwrap() as f32,
                clear[2].as_f64().unwrap() as f32,
                clear[3].as_f64().unwrap() as f32,
            ];
            let mut graph = RenderGraph::new(RenderTarget::Offscreen { color, width, height })
                .with_clear_color(clear_color);
            let mut commands = Vec::new();

            for operation in pass_spec["operations"].as_array().unwrap() {
                let pipeline_name = operation["pipeline"].as_str().unwrap();
                let pipeline = *pipelines.get(pipeline_name).expect("TC05 pipeline missing");
                let source = &operation["source"];
                let source_kind = source["kind"].as_str().unwrap();
                let loaded_asset = if source_kind == "asset" {
                    Some(h.load_texture(source["asset"].as_str().unwrap()))
                } else {
                    None
                };
                let texture_bind_group = if source_kind == "asset" {
                    loaded_asset.as_ref().unwrap().bind_group
                } else {
                    *target_bind_groups
                        .get(source["target"].as_str().unwrap())
                        .expect("TC05 source target bind group missing")
                };

                let mut draw = DrawCommand::new(
                    pipeline,
                    DrawAction::Procedural {
                        vertex_count: operation["vertex_count"].as_u64().unwrap() as u32,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, texture_bind_group, Vec::new());

                if operation["kind"].as_str().unwrap() == "sprite" {
                    let texture = loaded_asset.as_ref().expect("TC05 sprite asset missing");
                    let crop = operation["crop_uv"].as_array().unwrap();
                    let uniform = h.build_sprite_uniform(
                        texture,
                        array2(&operation["position"]),
                        operation["target_height_scale"].as_f64().unwrap() as f32,
                        [crop[0].as_f64().unwrap() as f32, crop[1].as_f64().unwrap() as f32],
                        [crop[2].as_f64().unwrap() as f32, crop[3].as_f64().unwrap() as f32],
                        operation["tolerance"].as_f64().unwrap() as f32,
                        operation["smoothness"].as_f64().unwrap() as f32,
                        operation["z_depth"].as_f64().unwrap() as f32,
                        operation["opacity"].as_f64().unwrap() as f32,
                    );
                    let uniform_bind_group = h.create_sprite_uniform_bind_group(uniform);
                    draw = draw.with_bind_group(1, uniform_bind_group, Vec::new());
                }
                commands.push(draw);
            }
            graph.add_batch(&mut h.pool, commands);
            graphs.push(graph);
        }

        let cold_render_time = execute_chain(&mut h, &graphs);
        let warm_render_time = execute_chain(&mut h, &graphs);

        let final_texture = target_textures.get("C").expect("TC05 final target missing");
        let output_path = "tests/outputs/desktop/tc05_interleaved.png";
        h.save_texture_to_file_checked(final_texture, wgpu::TextureFormat::Rgba8UnormSrgb, output_path)
            .expect("Failed to save TC05 output");
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
            .expect("Failed to read TC05 raw output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc05_interleaved_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC05",
            "manifest": "tests/shared_assets/manifests/tc05_interleaved.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của 3 pass + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "pass_count": graphs.len(),
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_render_time.as_secs_f64() * 1000.0
        });
        fs::write(
            "tests/outputs/desktop/tc05_interleaved_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
