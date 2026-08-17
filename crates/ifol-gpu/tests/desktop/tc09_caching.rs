mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
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

fn address_mode(value: &str) -> wgpu::AddressMode {
    match value {
        "repeat" => wgpu::AddressMode::Repeat,
        "mirror-repeat" => wgpu::AddressMode::MirrorRepeat,
        "clamp-to-edge" => wgpu::AddressMode::ClampToEdge,
        other => panic!("Unsupported TC09 sampler address mode: {other}"),
    }
}

fn filter_mode(value: &str) -> wgpu::FilterMode {
    match value {
        "nearest" => wgpu::FilterMode::Nearest,
        "linear" => wgpu::FilterMode::Linear,
        other => panic!("Unsupported TC09 sampler filter mode: {other}"),
    }
}

fn mipmap_filter_mode(value: &str) -> wgpu::MipmapFilterMode {
    match value {
        "nearest" => wgpu::MipmapFilterMode::Nearest,
        "linear" => wgpu::MipmapFilterMode::Linear,
        other => panic!("Unsupported TC09 sampler mipmap filter mode: {other}"),
    }
}

fn execute_and_wait(h: &mut DesktopTestHarness, graph: &RenderGraph) -> Duration {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .expect("TC09 graph execution failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed()
}

#[test]
fn run_tc09_caching() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc09_caching.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC09 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let warm_iterations = manifest["cache_contract"]["warm_iteration_count"]
            .as_u64()
            .unwrap() as usize;
        assert!(warm_iterations > 0);
        assert_eq!(graph_spec["command_count"], operations.len());

        let mut h = DesktopTestHarness::new(width, height).await;
        let sampler_spec = &graph_spec["sampler"];
        h.sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: address_mode(sampler_spec["address_mode_u"].as_str().unwrap()),
            address_mode_v: address_mode(sampler_spec["address_mode_v"].as_str().unwrap()),
            address_mode_w: address_mode(sampler_spec["address_mode_w"].as_str().unwrap()),
            mag_filter: filter_mode(sampler_spec["mag_filter"].as_str().unwrap()),
            min_filter: filter_mode(sampler_spec["min_filter"].as_str().unwrap()),
            mipmap_filter: mipmap_filter_mode(sampler_spec["mipmap_filter"].as_str().unwrap()),
            ..Default::default()
        });

        let background_spec = &graph_spec["pipelines"]["background"];
        let particle_spec = &graph_spec["pipelines"]["particles"];
        let background_pipeline = h.register_pipeline(
            background_spec["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::REPLACE),
            background_spec["depth"].as_bool().unwrap(),
            background_spec["has_uniform"].as_bool().unwrap(),
        );
        let particle_pipeline = h.register_pipeline(
            particle_spec["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            particle_spec["depth"].as_bool().unwrap(),
            particle_spec["has_uniform"].as_bool().unwrap(),
        );
        let background = h.load_texture(operations[0]["source"]["asset"].as_str().unwrap());
        let background_command = DrawCommand::new(
            background_pipeline,
            DrawAction::Procedural {
                vertex_count: operations[0]["vertex_count"].as_u64().unwrap() as u32,
                instance_range: 0..operations[0]["instance_count"].as_u64().unwrap() as u32,
            },
        )
        .with_bind_group(0, background.bind_group, Vec::new());
        let particle_command = DrawCommand::new(
            particle_pipeline,
            DrawAction::Procedural {
                vertex_count: operations[1]["vertex_count"].as_u64().unwrap() as u32,
                instance_range: 0..operations[1]["instance_count"].as_u64().unwrap() as u32,
            },
        );

        let (target_id, target_tex) = h.create_target("TC09 Target (Caching)");
        let clear = &graph_spec["clear_color"];
        let clear_color = [
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ];
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(clear_color);
        graph.add_batch(&mut h.pool, vec![background_command, particle_command]);

        let cold_time = execute_and_wait(&mut h, &graph);
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC09 cold raw output");
        let mut warm_times = Vec::with_capacity(warm_iterations);
        for _ in 0..warm_iterations {
            warm_times.push(execute_and_wait(&mut h, &graph));
        }
        let warm_total: Duration = warm_times.iter().copied().sum();
        let warm_average = warm_total / warm_iterations as u32;

        h.save_texture_to_file_checked(&target_tex, format, "tests/outputs/desktop/tc09_caching.png")
            .expect("Failed to save TC09 output");
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC09 raw output");
        assert_eq!(cold_raw.bytes, raw.bytes, "TC09 cache changed rendered output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc09_caching_desktop.bin", &raw.bytes).unwrap();

        let cold_ms = cold_time.as_secs_f64() * 1000.0;
        let warm_ms = warm_average.as_secs_f64() * 1000.0;
        let metadata = serde_json::json!({
            "test_case": "TC09",
            "manifest": "tests/shared_assets/manifests/tc09_caching.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của cùng graph + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "instance_count": operations[1]["instance_count"],
            "warm_iteration_count": warm_iterations,
            "cache_output_equal": cold_raw.bytes == raw.bytes,
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_ms,
            "warm_render_time_ms": warm_ms,
            "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0
        });
        fs::write(
            "tests/outputs/desktop/tc09_caching_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
