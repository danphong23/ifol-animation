mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RippleUniform {
    center: [f32; 2],
    time: f32,
    amplitude: f32,
    frequency: f32,
    speed: f32,
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
        .expect("TC28 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC28 value must be numeric") as f32)
        .collect()
}

fn value2(value: &Value) -> [f32; 2] {
    values(value)
        .try_into()
        .expect("TC28 value must have two elements")
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC28 value must have four elements")
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC28 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

fn ensure_canonical_city_fixture() {
    let source = Path::new("tests/shared_assets/textures/bg_anime_city.jpg");
    let canonical = Path::new("tests/shared_assets/textures/canonical_bg_anime_city.png");
    if !canonical.exists() {
        image::open(source)
            .expect("TC28 source JPEG could not be decoded")
            .save_with_format(canonical, image::ImageFormat::Png)
            .expect("TC28 canonical PNG fixture could not be written");
    }
}

#[test]
fn run_tc28_ripple() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc28_ripple.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC28 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operation = &graph_spec["operations"][0];
        let pass = &graph_spec["passes"][0];
        let u = &operation["uniform"];

        ensure_canonical_city_fixture();
        let mut h = DesktopTestHarness::new(width, height).await;
        let city = h.load_texture_exact("canonical_bg_anime_city.png");
        let pipeline =
            h.register_pipeline("ripple.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let uniform = h.create_custom_uniform_bind_group(
            RippleUniform {
                center: value2(&u["center"]),
                time: u["time"].as_f64().unwrap() as f32,
                amplitude: u["amplitude"].as_f64().unwrap() as f32,
                frequency: u["frequency"].as_f64().unwrap() as f32,
                speed: u["speed"].as_f64().unwrap() as f32,
                _pad: [0.0; 2],
            },
            "TC28 Ripple Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC28 Ripple");
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
            .with_bind_group(0, city.bind_group, Vec::new())
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
            .expect("TC28 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC28 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC28 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc28_ripple.png",
        )
        .expect("TC28 PNG save failed");
        fs::write("tests/outputs/desktop/tc28_ripple_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC28",
            "manifest": "tests/shared_assets/manifests/tc28_ripple.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (radial ripple UV distortion) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
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
            "tests/outputs/desktop/tc28_ripple_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
