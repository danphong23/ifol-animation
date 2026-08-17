mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GodRaysUniform {
    light_pos: [f32; 2],
    exposure: f32,
    decay: f32,
    density: f32,
    weight: f32,
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
        .expect("TC27 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC27 value must be numeric") as f32)
        .collect()
}

fn value2(value: &Value) -> [f32; 2] {
    values(value)
        .try_into()
        .expect("TC27 value must have two elements")
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC27 value must have four elements")
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC27 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc27_godrays() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc27_godrays.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC27 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operation = &graph_spec["operations"][0];
        let pass = &graph_spec["passes"][0];
        let u = &operation["uniform"];

        let mut h = DesktopTestHarness::new(width, height).await;
        let background = h.load_texture_exact("canonical_bg_forest.png");
        let pipeline = h.register_pipeline(
            "godrays.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let uniform = h.create_custom_uniform_bind_group(
            GodRaysUniform {
                light_pos: value2(&u["light_pos"]),
                exposure: u["exposure"].as_f64().unwrap() as f32,
                decay: u["decay"].as_f64().unwrap() as f32,
                density: u["density"].as_f64().unwrap() as f32,
                weight: u["weight"].as_f64().unwrap() as f32,
                _pad: [0.0; 2],
            },
            "TC27 Godrays Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC27 Godrays");
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
            .expect("TC27 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC27 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC27 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc27_godrays.png",
        )
        .expect("TC27 PNG save failed");
        fs::write("tests/outputs/desktop/tc27_godrays_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC27",
            "manifest": "tests/shared_assets/manifests/tc27_godrays.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (100-sample radial godrays accumulation) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
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
            "tests/outputs/desktop/tc27_godrays_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
