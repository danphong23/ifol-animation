mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AudioUniform {
    freqs: [[f32; 4]; 4],
    base_color: [f32; 4],
    time: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
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
        .expect("TC19 value must be an array")
        .iter()
        .map(|item| item.as_f64().expect("TC19 value must be numeric") as f32)
        .collect()
}

fn operation<'a>(operations: &'a [Value], id: &str) -> &'a Value {
    operations
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("Missing TC19 operation: {id}"))
}

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC19 graph pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc19_audio_viz() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc19_audio_viz.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC19 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let passes = graph_spec["passes"].as_array().unwrap();
        let audio_spec = operation(operations, "audio_visualizer");
        let freqs: Vec<f32> = values(&audio_spec["uniform"]["freqs"]);
        let base_color: [f32; 4] = values(&audio_spec["uniform"]["base_color"])
            .try_into()
            .expect("TC19 base color must have four values");
        let freqs: [[f32; 4]; 4] = freqs
            .chunks_exact(4)
            .map(|chunk| [chunk[0], chunk[1], chunk[2], chunk[3]])
            .collect::<Vec<_>>()
            .try_into()
            .expect("TC19 requires sixteen frequencies");
        let mut h = DesktopTestHarness::new(width, height).await;
        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let audio_pipeline = h.register_pipeline(
            "audio_spectrum.wgsl",
            Some(wgpu::BlendState::REPLACE),
            false,
            true,
        );
        let audio_uniform = h.create_custom_uniform_bind_group(
            AudioUniform {
                freqs,
                base_color,
                time: audio_spec["uniform"]["time"].as_f64().unwrap() as f32,
                _pad1: 0.0,
                _pad2: 0.0,
                _pad3: 0.0,
            },
            "TC19 Audio Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC19 Audio Spectrum");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(
            values(&passes[0]["clear_color"])
                .try_into()
                .expect("TC19 clear color must have four values"),
        );
        graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                audio_pipeline,
                DrawAction::Procedural {
                    vertex_count: audio_spec["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..audio_spec["instance_count"].as_u64().unwrap() as u32,
                },
            )
            .with_bind_group(0, noise.bind_group, Vec::new())
            .with_bind_group(1, audio_uniform, Vec::new())],
        );

        fs::create_dir_all("tests/outputs/desktop").unwrap();
        let cold_render_time_ms = execute(&mut h, &graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC19 cold readback failed");
        let warm_render_time_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC19 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC19 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            "tests/outputs/desktop/tc19_audio_viz.png",
        )
        .expect("TC19 PNG save failed");
        fs::write(
            "tests/outputs/desktop/tc19_audio_viz_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC19",
            "manifest": "tests/shared_assets/manifests/tc19_audio_viz.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "1 pass (audio spectrum visualizer) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "pass_count": passes.len(),
            "instance_count": audio_spec["instance_count"],
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time_ms,
            "warm_render_time_ms": warm_render_time_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_render_time_ms / cold_render_time_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc19_audio_viz_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
