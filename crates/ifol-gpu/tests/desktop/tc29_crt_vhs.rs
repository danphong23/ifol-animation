mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CrtUniform {
    curvature: [f32; 2],
    scanline_intensity: f32,
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
        .expect("TC29 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC29 value must be numeric") as f32)
        .collect()
}

fn value2(value: &Value) -> [f32; 2] {
    values(value)
        .try_into()
        .expect("TC29 value must have two elements")
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC29 value must have four elements")
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC29 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc29_crt_vhs() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc29_crt_vhs.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC29 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operation = &graph_spec["operations"][0];
        let pass = &graph_spec["passes"][0];
        let u = &operation["uniform"];

        let mut h = DesktopTestHarness::new(width, height).await;
        let background = h.load_texture_exact("canonical_bg_scifi.png");
        let pipeline =
            h.register_pipeline("crt_vhs.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let uniform = h.create_custom_uniform_bind_group(
            CrtUniform {
                curvature: value2(&u["curvature"]),
                scanline_intensity: u["scanline_intensity"].as_f64().unwrap() as f32,
                time: u["time"].as_f64().unwrap() as f32,
            },
            "TC29 CRT Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC29 CRT VHS");
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
            .with_bind_group(0, background.bind_group, Vec::new())
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
            .expect("TC29 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC29 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC29 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc29_crt_vhs.png",
        )
        .expect("TC29 PNG save failed");
        fs::write("tests/outputs/desktop/tc29_crt_vhs_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC29",
            "manifest": "tests/shared_assets/manifests/tc29_crt_vhs.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (CRT curvature + scanlines + vignette + RGB split + integer-hash noise) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
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
            "tests/outputs/desktop/tc29_crt_vhs_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
