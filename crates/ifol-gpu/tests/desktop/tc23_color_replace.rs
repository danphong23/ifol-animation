mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ReplaceUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    target_hsv: [f32; 4],
    replace_hsv: [f32; 4],
    tolerance: f32,
    smoothness: f32,
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

fn values(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("TC23 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC23 value must be numeric") as f32)
        .collect()
}

fn value2(value: &Value) -> [f32; 2] {
    values(value)
        .try_into()
        .expect("TC23 value must have two elements")
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC23 value must have four elements")
}

fn matrix4(value: &Value) -> [[f32; 4]; 4] {
    value
        .as_array()
        .expect("TC23 matrix must be an array")
        .iter()
        .map(|column| value4(column))
        .collect::<Vec<_>>()
        .try_into()
        .expect("TC23 matrix must have four columns")
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC23 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc23_color_replace() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc23_color_replace.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC23 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operation = &graph_spec["operations"][0];
        let pass = &graph_spec["passes"][0];
        let u = &operation["uniform"];
        let mut h = DesktopTestHarness::new(width, height).await;
        let sprite = h.load_texture_exact("canonical_sprites_heroes.png");
        let pipeline = h.register_pipeline(
            "color_replace.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let uniform = h.create_custom_uniform_bind_group(
            ReplaceUniform {
                transform: matrix4(&u["transform"]),
                uv_min: value2(&u["uv_min"]),
                uv_max: value2(&u["uv_max"]),
                target_hsv: value4(&u["target_hsv"]),
                replace_hsv: value4(&u["replace_hsv"]),
                tolerance: u["tolerance"].as_f64().unwrap() as f32,
                smoothness: u["smoothness"].as_f64().unwrap() as f32,
                _pad: [0.0; 2],
            },
            "TC23 Replace Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC23 Palette Swap");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(value4(&pass["clear_color"]));
        graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                pipeline,
                DrawAction::Procedural {
                    vertex_count: operation["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..operation["instance_count"].as_u64().unwrap() as u32,
                },
            )
            .with_bind_group(0, sprite.bind_group, Vec::new())
            .with_bind_group(1, uniform, Vec::new())],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute(&mut h, &graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC23 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC23 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC23 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc23_color_replace.png",
        )
        .expect("TC23 PNG save failed");
        fs::write(
            "tests/outputs/desktop/tc23_color_replace_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC23",
            "manifest": "tests/shared_assets/manifests/tc23_color_replace.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (HSV palette replacement + chroma key) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": graph_spec["passes"].as_array().unwrap().len(),
            "instance_count": operation["instance_count"],
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc23_color_replace_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
