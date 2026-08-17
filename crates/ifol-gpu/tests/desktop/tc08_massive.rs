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

#[test]
fn run_tc08_massive() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc08_massive.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC08 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        let mut h = DesktopTestHarness::new(width, height).await;
        h.sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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

        let (target_id, target_tex) = h.create_target("TC08 Massive Target");
        let clear = &graph_spec["clear_color"];
        let clear_color = [
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ];
        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_id, width, height })
            .with_clear_color(clear_color);
        graph.add_batch(&mut h.pool, vec![background_command, particle_command]);

        let cold_start = Instant::now();
        let cold_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph)
            .expect("TC08 cold execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(cold_submission),
            timeout: None,
        });
        let cold_render_time = cold_start.elapsed();

        let warm_start = Instant::now();
        let warm_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph)
            .expect("TC08 warm execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(warm_submission),
            timeout: None,
        });
        let warm_render_time = warm_start.elapsed();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        h.save_texture_to_file_checked(&target_tex, format, "tests/outputs/desktop/tc08_massive.png")
            .expect("Failed to save TC08 output");
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC08 raw output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc08_massive_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC08",
            "manifest": "tests/shared_assets/manifests/tc08_massive.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của graph 1 node/2 draw command với 10.000 instance + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "instance_count": operations[1]["instance_count"],
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_render_time.as_secs_f64() * 1000.0
        });
        fs::write(
            "tests/outputs/desktop/tc08_massive_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
