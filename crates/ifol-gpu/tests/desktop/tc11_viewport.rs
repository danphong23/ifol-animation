mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn execute_pass(h: &mut DesktopTestHarness, graph: &RenderGraph) {
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC11 viewport execution failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}

fn execute_all(
    h: &mut DesktopTestHarness,
    left: &RenderGraph,
    right: &RenderGraph,
    final_graph: &RenderGraph,
) -> f64 {
    let started = Instant::now();
    execute_pass(h, left);
    execute_pass(h, right);
    execute_pass(h, final_graph);
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc11_viewport() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc11_viewport.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC11 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let left_spec = &graph_spec["targets"]["left"];
        let right_spec = &graph_spec["targets"]["right"];
        let mut h = DesktopTestHarness::new(width, height).await;
        h.sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let (left_target_id, _) = h.create_custom_target(
            left_spec["width"].as_u64().unwrap() as u32,
            left_spec["height"].as_u64().unwrap() as u32,
            "TC11 Left Viewport",
        );
        let left_view = h.create_texture_bind_group(left_target_id, "TC11 Left View");
        let (right_target_id, _) = h.create_custom_target(
            right_spec["width"].as_u64().unwrap() as u32,
            right_spec["height"].as_u64().unwrap() as u32,
            "TC11 Right Viewport",
        );
        let right_view = h.create_texture_bind_group(right_target_id, "TC11 Right View");
        let (final_target_id, final_target_tex) = h.create_target("TC11 Final Split Screen");

        let left_clear = &graph_spec["passes"][0]["clear_color"];
        let right_clear = &graph_spec["passes"][1]["clear_color"];
        let final_clear = &graph_spec["passes"][2]["clear_color"];
        let color = |value: &Value| {
            [
                value[0].as_f64().unwrap() as f32,
                value[1].as_f64().unwrap() as f32,
                value[2].as_f64().unwrap() as f32,
                value[3].as_f64().unwrap() as f32,
            ]
        };
        let left_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: left_target_id,
            width: 400,
            height: 600,
        })
        .with_clear_color(color(left_clear));
        let right_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: right_target_id,
            width: 400,
            height: 600,
        })
        .with_clear_color(color(right_clear));
        let split_pipeline = h.register_splitscreen_pipeline();
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width,
            height,
        })
        .with_clear_color(color(final_clear));
        final_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                split_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, left_view, Vec::new())
            .with_bind_group(1, right_view, Vec::new())],
        );

        let cold_render_time_ms = execute_all(&mut h, &left_graph, &right_graph, &final_graph);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_target_tex, format)
            .expect("Failed to read TC11 cold output");
        let warm_render_time_ms = execute_all(&mut h, &left_graph, &right_graph, &final_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&final_target_tex, format)
            .expect("Failed to read TC11 output");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC11 viewport output changed between runs"
        );

        h.save_texture_to_file_checked(
            &final_target_tex,
            format,
            "tests/outputs/desktop/tc11_viewport.png",
        )
        .expect("Failed to save TC11 output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write(
            "tests/outputs/desktop/tc11_viewport_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC11",
            "manifest": "tests/shared_assets/manifests/tc11_viewport.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "3 pass execute_checked (left → right → final) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": graph_spec["passes"].as_array().unwrap().len(),
            "viewport_count": 2,
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc11_viewport_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
