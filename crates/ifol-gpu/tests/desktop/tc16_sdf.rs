mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SdfShapeUniform {
    shape_type: f32,
    size_x: f32,
    size_y: f32,
    corner_radius: f32,
    color: [f32; 4],
    border_color: [f32; 4],
    border_width: f32,
    glow_strength: f32,
    pos_x: f32,
    pos_y: f32,
    rotation: f32,
    scale: f32,
    aspect_ratio: f32,
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

fn values(value: &Value) -> Vec<f32> {
    value
        .as_array()
        .expect("TC16 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC16 value must be numeric") as f32)
        .collect()
}

fn value4(value: &Value) -> [f32; 4] {
    values(value)
        .try_into()
        .expect("TC16 value must have four elements")
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC16 operation: {id}"))
}

fn uniform_from_spec(spec: &Value, aspect_ratio: f32) -> SdfShapeUniform {
    let uniform = &spec["uniform"];
    let position = values(&uniform["position"]);
    SdfShapeUniform {
        shape_type: uniform["shape_type"].as_f64().unwrap() as f32,
        size_x: uniform["size_x"].as_f64().unwrap() as f32,
        size_y: uniform["size_y"].as_f64().unwrap() as f32,
        corner_radius: uniform["corner_radius"].as_f64().unwrap() as f32,
        color: value4(&uniform["color"]),
        border_color: value4(&uniform["border_color"]),
        border_width: uniform["border_width"].as_f64().unwrap() as f32,
        glow_strength: uniform["glow_strength"].as_f64().unwrap() as f32,
        pos_x: position[0],
        pos_y: position[1],
        rotation: uniform["rotation"].as_f64().unwrap() as f32,
        scale: uniform["scale"].as_f64().unwrap() as f32,
        aspect_ratio,
        _pad: 0.0,
    }
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC16 graph execution failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc16_sdf() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc16_sdf.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC16 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let clear_color: [f32; 4] = value4(&graph_spec["clear_color"]);
        let mut h = DesktopTestHarness::new(width, height).await;

        let uniform_layout = h.uniform_bg_layout.clone();
        let sdf_pipeline = h.register_custom_pipeline(
            "sdf_shapes.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            vec![Some(2)],
            vec![Some(&uniform_layout)],
        );
        let aspect_ratio = width as f32 / height as f32;
        let circle_bg = h.create_custom_uniform_bind_group(
            uniform_from_spec(operation(operations, "circle"), aspect_ratio),
            "TC16 Circle Uniform",
        );
        let rounded_rect_bg = h.create_custom_uniform_bind_group(
            uniform_from_spec(operation(operations, "rounded_rect"), aspect_ratio),
            "TC16 Rounded Rect Uniform",
        );
        let ring_bg = h.create_custom_uniform_bind_group(
            uniform_from_spec(operation(operations, "ring"), aspect_ratio),
            "TC16 Ring Uniform",
        );
        let triangle_bg = h.create_custom_uniform_bind_group(
            uniform_from_spec(operation(operations, "triangle"), aspect_ratio),
            "TC16 Triangle Uniform",
        );

        let (target_id, target_texture) = h.create_target("TC16 SDF Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(clear_color);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sdf_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, circle_bg, Vec::new()),
                DrawCommand::new(
                    sdf_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, rounded_rect_bg, Vec::new()),
                DrawCommand::new(
                    sdf_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, ring_bg, Vec::new()),
                DrawCommand::new(
                    sdf_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, triangle_bg, Vec::new()),
            ],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute(&mut h, &graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC16 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC16 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC16 output changed between runs"
        );

        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc16_sdf.png",
        )
        .expect("TC16 PNG save failed");
        fs::write("tests/outputs/desktop/tc16_sdf_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC16",
            "manifest": "tests/shared_assets/manifests/tc16_sdf.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (2D SDF scene, 4 draw commands) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": graph_spec["passes"].as_array().unwrap().len(),
            "shape_count": manifest["evaluation"]["expected_shape_count"],
            "instance_count": operations.iter().map(|operation| operation["instance_count"].as_u64().unwrap()).sum::<u64>(),
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc16_sdf_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
