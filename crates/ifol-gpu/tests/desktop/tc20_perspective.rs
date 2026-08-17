mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PerspectiveUniform {
    mvp: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    key_color: [f32; 3],
    tolerance: f32,
    smoothness: f32,
    opacity: f32,
    _pad1: f32,
    _pad2: f32,
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
        .expect("TC20 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC20 value must be numeric") as f32)
        .collect()
}

fn value2(value: &Value) -> [f32; 2] {
    values(value)
        .try_into()
        .expect("TC20 value must have two elements")
}

fn value3(value: &Value) -> [f32; 3] {
    values(value)
        .try_into()
        .expect("TC20 value must have three elements")
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC20 value must have four elements")
}

fn matrix4(value: &Value) -> [[f32; 4]; 4] {
    value
        .as_array()
        .expect("TC20 matrix must be an array")
        .iter()
        .map(|column| value4(column))
        .collect::<Vec<_>>()
        .try_into()
        .expect("TC20 matrix must have four columns")
}

fn fnv_operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC20 operation: {id}"))
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC20 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc20_perspective() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc20_perspective.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC20 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let passes = graph_spec["passes"].as_array().unwrap();
        let spec = fnv_operation(operations, "perspective_card");
        let u = &spec["uniform"];
        let mut h = DesktopTestHarness::new(width, height).await;
        let sprite = h.load_texture_exact("canonical_sprites_heroes.png");
        let pipeline = h.register_pipeline(
            "perspective_sprite.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let uniform = h.create_custom_uniform_bind_group(
            PerspectiveUniform {
                mvp: matrix4(&u["mvp"]),
                uv_min: value2(&u["uv_min"]),
                uv_max: value2(&u["uv_max"]),
                key_color: value3(&u["key_color"]),
                tolerance: u["tolerance"].as_f64().unwrap() as f32,
                smoothness: u["smoothness"].as_f64().unwrap() as f32,
                opacity: u["opacity"].as_f64().unwrap() as f32,
                _pad1: 0.0,
                _pad2: 0.0,
            },
            "TC20 Perspective Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC20 Perspective Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(value4(&passes[0]["clear_color"]));
        graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                pipeline,
                DrawAction::Procedural {
                    vertex_count: spec["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..spec["instance_count"].as_u64().unwrap() as u32,
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
            .expect("TC20 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC20 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC20 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc20_perspective.png",
        )
        .expect("TC20 PNG save failed");
        fs::write(
            "tests/outputs/desktop/tc20_perspective_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC20",
            "manifest": "tests/shared_assets/manifests/tc20_perspective.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (fixed MVP perspective sprite) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": passes.len(),
            "instance_count": spec["instance_count"],
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc20_perspective_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
