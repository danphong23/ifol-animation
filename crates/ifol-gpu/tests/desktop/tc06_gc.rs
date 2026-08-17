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

fn array2(value: &Value) -> [f32; 2] {
    [value[0].as_f64().unwrap() as f32, value[1].as_f64().unwrap() as f32]
}

#[test]
fn run_tc06_gc() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc06_gc.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC06 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;

        let pipeline_spec = &graph_spec["pipeline"];
        let pipeline = h.register_pipeline(
            pipeline_spec["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            pipeline_spec["depth"].as_bool().unwrap(),
            pipeline_spec["has_uniform"].as_bool().unwrap(),
        );
        let operation = &graph_spec["operation"];
        let texture = h.load_texture(operation["source"]["asset"].as_str().unwrap());
        let crop = operation["crop_uv"].as_array().unwrap();
        let uniform = h.build_sprite_uniform(
            &texture,
            array2(&operation["position"]),
            operation["target_height_scale"].as_f64().unwrap() as f32,
            [crop[0].as_f64().unwrap() as f32, crop[1].as_f64().unwrap() as f32],
            [crop[2].as_f64().unwrap() as f32, crop[3].as_f64().unwrap() as f32],
            operation["tolerance"].as_f64().unwrap() as f32,
            operation["smoothness"].as_f64().unwrap() as f32,
            operation["z_depth"].as_f64().unwrap() as f32,
            operation["opacity"].as_f64().unwrap() as f32,
        );
        let uniform_bind_group = h.create_sprite_uniform_bind_group(uniform);
        let command = DrawCommand::new(
            pipeline,
            DrawAction::Procedural {
                vertex_count: operation["vertex_count"].as_u64().unwrap() as u32,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, texture.bind_group, Vec::new())
        .with_bind_group(1, uniform_bind_group, Vec::new());

        let pool_spec = &graph_spec["node_pool"];
        let allocated = pool_spec["allocated"].as_u64().unwrap() as usize;
        let freed = pool_spec["freed"].as_u64().unwrap() as usize;
        let surviving = pool_spec["surviving"].as_u64().unwrap() as usize;
        let mut node_ids = Vec::with_capacity(allocated);
        for _ in 0..allocated {
            node_ids.push(h.pool.alloc_batch(vec![command.clone()]));
        }
        assert_eq!(h.pool.len(), allocated, "TC06 allocation count mismatch");
        for id in node_ids.iter().take(freed) {
            h.pool.remove(*id);
        }
        assert_eq!(h.pool.len(), surviving, "TC06 surviving node count mismatch");

        let (target_id, target_tex) = h.create_target("TC06 Target");
        let clear = &graph_spec["clear_color"];
        let clear_color = [
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ];
        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_id, width, height })
            .with_clear_color(clear_color);
        graph.add_node_id(*node_ids.last().expect("TC06 survivor missing"));

        let cold_start = Instant::now();
        let cold_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph)
            .expect("TC06 cold execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(cold_submission),
            timeout: None,
        });
        let cold_render_time = cold_start.elapsed();

        let warm_start = Instant::now();
        let warm_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph)
            .expect("TC06 warm execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(warm_submission),
            timeout: None,
        });
        let warm_render_time = warm_start.elapsed();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        h.save_texture_to_file_checked(&target_tex, format, "tests/outputs/desktop/tc06_gc.png")
            .expect("Failed to save TC06 output");
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC06 raw output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc06_gc_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC06",
            "manifest": "tests/shared_assets/manifests/tc06_gc.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của graph còn một node + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "allocated_nodes": allocated,
            "freed_nodes": freed,
            "surviving_nodes": h.pool.len(),
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_render_time.as_secs_f64() * 1000.0
        });
        fs::write(
            "tests/outputs/desktop/tc06_gc_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
