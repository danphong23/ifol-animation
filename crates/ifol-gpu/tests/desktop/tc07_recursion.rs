mod harness;

use harness::{DesktopTestHarness, LoadedTextureInfo};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::PipelineHandle;
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

fn sprite_command(
    h: &mut DesktopTestHarness<'_>,
    pipeline: PipelineHandle,
    texture: &LoadedTextureInfo,
    operation: &Value,
) -> DrawCommand {
    let crop = operation["crop_uv"].as_array().unwrap();
    let uniform = h.build_sprite_uniform(
        texture,
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
    DrawCommand::new(
        pipeline,
        DrawAction::Procedural {
            vertex_count: operation["vertex_count"].as_u64().unwrap() as u32,
            instance_range: 0..1,
        },
    )
    .with_bind_group(0, texture.bind_group, Vec::new())
    .with_bind_group(1, uniform_bind_group, Vec::new())
}

#[test]
fn run_tc07_recursion() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc07_recursion.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC07 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let operations = graph_spec["operations"].as_array().unwrap();
        assert_eq!(graph_spec["depth"], 5);
        assert_eq!(operations.len(), 5);

        let mut h = DesktopTestHarness::new(width, height).await;
        h.sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let background_spec = &graph_spec["pipelines"]["background"];
        let sprite_spec = &graph_spec["pipelines"]["sprite"];
        let background_pipeline = h.register_pipeline(
            background_spec["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::REPLACE),
            background_spec["depth"].as_bool().unwrap(),
            background_spec["has_uniform"].as_bool().unwrap(),
        );
        let sprite_pipeline = h.register_pipeline(
            sprite_spec["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            sprite_spec["depth"].as_bool().unwrap(),
            sprite_spec["has_uniform"].as_bool().unwrap(),
        );

        let background_texture = h.load_texture(operations[0]["source"]["asset"].as_str().unwrap());
        let tree_texture = h.load_texture(operations[1]["source"]["asset"].as_str().unwrap());
        let golem_texture = h.load_texture(operations[2]["source"]["asset"].as_str().unwrap());
        let wizard_texture = h.load_texture(operations[3]["source"]["asset"].as_str().unwrap());
        let chest_texture = h.load_texture(operations[4]["source"]["asset"].as_str().unwrap());

        let background_command = DrawCommand::new(
            background_pipeline,
            DrawAction::Procedural {
                vertex_count: operations[0]["vertex_count"].as_u64().unwrap() as u32,
                instance_range: 0..1,
            },
        )
        .with_bind_group(0, background_texture.bind_group, Vec::new());
        let tree_command = sprite_command(&mut h, sprite_pipeline, &tree_texture, &operations[1]);
        let golem_command = sprite_command(&mut h, sprite_pipeline, &golem_texture, &operations[2]);
        let wizard_command = sprite_command(&mut h, sprite_pipeline, &wizard_texture, &operations[3]);
        let chest_command = sprite_command(&mut h, sprite_pipeline, &chest_texture, &operations[4]);

        let (target_id, target_tex) = h.create_target("TC07 Recursion Target");
        let clear = &graph_spec["clear_color"];
        let clear_color = [
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ];
        let target = || RenderTarget::Offscreen { color: target_id, width, height };

        let mut graph_e = RenderGraph::new(target()).with_clear_color(clear_color);
        graph_e.add_node_id(h.pool.alloc_batch(vec![background_command]));

        let mut graph_d = RenderGraph::new(target());
        graph_d.add_node_id(h.pool.alloc_subgraph("SubGraph E (Background)", graph_e, vec![tree_command]));

        let mut graph_c = RenderGraph::new(target());
        graph_c.add_node_id(h.pool.alloc_subgraph("SubGraph D (Tree)", graph_d, vec![golem_command]));

        let mut graph_b = RenderGraph::new(target());
        graph_b.add_node_id(h.pool.alloc_subgraph("SubGraph C (Golem)", graph_c, vec![wizard_command]));

        let mut graph_a = RenderGraph::new(target());
        graph_a.add_node_id(h.pool.alloc_subgraph("SubGraph B (Wizard)", graph_b, vec![chest_command]));

        let cold_start = Instant::now();
        let cold_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph_a)
            .expect("TC07 cold execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(cold_submission),
            timeout: None,
        });
        let cold_render_time = cold_start.elapsed();

        let warm_start = Instant::now();
        let warm_submission = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut h.pool, &graph_a)
            .expect("TC07 warm execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(warm_submission),
            timeout: None,
        });
        let warm_render_time = warm_start.elapsed();

        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        h.save_texture_to_file_checked(&target_tex, format, "tests/outputs/desktop/tc07_recursion.png")
            .expect("Failed to save TC07 output");
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(&target_tex, format)
            .expect("Failed to read TC07 raw output");
        fs::create_dir_all("tests/outputs/desktop").unwrap();
        fs::write("tests/outputs/desktop/tc07_recursion_desktop.bin", &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC07",
            "manifest": "tests/shared_assets/manifests/tc07_recursion.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "width": raw.width,
            "height": raw.height,
            "format": format!("{format:?}"),
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "execute_checked của graph lồng 5 cấp + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "recursion_depth": graph_spec["depth"],
            "flattened_operations": operations.len(),
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "cold_render_time_ms": cold_render_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_render_time.as_secs_f64() * 1000.0
        });
        fs::write(
            "tests/outputs/desktop/tc07_recursion_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
