mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::execution::RenderGraphValidationError;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::BindGroupHandle;
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

fn execute_and_wait(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC10 fallback execution failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc10_fallback() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc10_fallback.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC10 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let missing_handle = manifest["error_contract"]["missing_bind_group"].as_u64().unwrap();
        let fallback = &graph_spec["clear_color"];
        let fallback_color = [
            fallback[0].as_f64().unwrap() as f32,
            fallback[1].as_f64().unwrap() as f32,
            fallback[2].as_f64().unwrap() as f32,
            fallback[3].as_f64().unwrap() as f32,
        ];
        let mut h = DesktopTestHarness::new(width, height).await;
        let pipe_id = h.register_pipeline("chroma_key.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let (target_id, target_tex) = h.create_target("TC10 Fallback Target");

        let mut bad_graph = RenderGraph::new(RenderTarget::Offscreen { color: target_id, width, height });
        bad_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipe_id,
                    DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 },
                )
                .with_bind_group(0, BindGroupHandle(missing_handle), Vec::new()),
            ],
        );
        let validation_result = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &mut bad_graph);
        let validation_error = validation_result.expect_err("TC10 must reject the missing bind group");
        assert_eq!(
            validation_error,
            RenderGraphValidationError::MissingBindGroup(BindGroupHandle(missing_handle))
        );

        let fallback_graph = RenderGraph::new(RenderTarget::Offscreen { color: target_id, width, height })
            .with_clear_color(fallback_color);
        let cold_render_time_ms = execute_and_wait(&mut h, &fallback_graph);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC10 cold fallback output");
        let warm_render_time_ms = execute_and_wait(&mut h, &fallback_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC10 fallback output");
        assert_eq!(cold_raw.bytes, raw.bytes, "TC10 fallback changed between cold and warm");
        let expected_pixel = [255u8, 0, 255, 255];
        assert_eq!(raw.bytes, expected_pixel.repeat((width * height) as usize));

        h.save_texture_to_file_checked(&target_tex, format, "tests/outputs/desktop/tc10_fallback.png")
            .expect("Failed to save TC10 output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc10_fallback_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC10",
            "manifest": "tests/shared_assets/manifests/tc10_fallback.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "fallback graph execute_checked + submit queue + device.poll(Wait); không gồm validation graph lỗi, khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "validation_error": "RenderGraphValidationError::MissingBindGroup",
            "missing_bind_group": missing_handle,
            "validation_passed": true,
            "panic_occurred": false,
            "fallback_color": [255, 0, 255, 255],
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms
        });
        fs::write(
            "tests/outputs/desktop/tc10_fallback_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
