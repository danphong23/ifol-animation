mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DissolveUniform {
    threshold: f32,
    edge_width: f32,
    _pad0: [f32; 2],
    edge_color: [f32; 3],
    _pad1: f32,
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn execute_graph_pair(
    h: &mut DesktopTestHarness,
    chroma_graph: &RenderGraph,
    dissolve_graph: &RenderGraph,
) -> f64 {
    let started = Instant::now();
    h.executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, chroma_graph)
        .expect("TC30 chroma pass failed");
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, dissolve_graph)
        .expect("TC30 dissolve pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc30_dissolve() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc30_dissolve.json");
        let manifest: serde_json::Value = serde_json::from_str(manifest_text).unwrap();
        let graph_spec = &manifest["graph"];
        let operation_chroma = &graph_spec["operations"][0];
        let operation_dissolve = &graph_spec["operations"][1];
        let width = graph_spec["target"]["width"].as_u64().unwrap() as u32;
        let height = graph_spec["target"]["height"].as_u64().unwrap() as u32;

        let mut h = DesktopTestHarness::new(width, height).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let noise = h.load_texture_exact("canonical_tc085_noise.png");
        let pipe_chroma = h.register_pipeline(
            operation_chroma["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let pipe_dissolve = h.register_dual_texture_pipeline(
            operation_dissolve["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );

        let screen_aspect = width as f64 / height as f64;
        let crop = &operation_chroma["uniform"];
        let p_scale_y = crop["scale_y"].as_f64().unwrap();
        let p_crop_w = (crop["uv_max"][0].as_f64().unwrap() - crop["uv_min"][0].as_f64().unwrap())
            * heroes.width as f64;
        let p_crop_h = (crop["uv_max"][1].as_f64().unwrap() - crop["uv_min"][1].as_f64().unwrap())
            * heroes.height as f64;
        let p_scale_x = (p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect)) as f32;
        let sprite_uniform = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [p_scale_x, p_scale_y as f32],
            uv_min: [
                crop["uv_min"][0].as_f64().unwrap() as f32,
                crop["uv_min"][1].as_f64().unwrap() as f32,
            ],
            uv_max: [
                crop["uv_max"][0].as_f64().unwrap() as f32,
                crop["uv_max"][1].as_f64().unwrap() as f32,
            ],
            key_color: [
                crop["key_color"][0].as_f64().unwrap() as f32,
                crop["key_color"][1].as_f64().unwrap() as f32,
                crop["key_color"][2].as_f64().unwrap() as f32,
            ],
            tolerance: crop["tolerance"].as_f64().unwrap() as f32,
            smoothness: crop["smoothness"].as_f64().unwrap() as f32,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let sprite_bind_group = h.create_custom_uniform_bind_group(sprite_uniform, "TC30 Chroma");
        let dissolve = &operation_dissolve["uniform"];
        let dissolve_uniform = DissolveUniform {
            threshold: dissolve["threshold"].as_f64().unwrap() as f32,
            edge_width: dissolve["edge_width"].as_f64().unwrap() as f32,
            _pad0: [0.0, 0.0],
            edge_color: [
                dissolve["edge_color"][0].as_f64().unwrap() as f32,
                dissolve["edge_color"][1].as_f64().unwrap() as f32,
                dissolve["edge_color"][2].as_f64().unwrap() as f32,
            ],
            _pad1: 0.0,
        };
        let dissolve_bind_group =
            h.create_custom_uniform_bind_group(dissolve_uniform, "TC30 Dissolve");

        let (target_a_id, _target_a_texture) = h.create_target("TC30 Chroma Target");
        let (final_id, final_texture) = h.create_target("TC30 Final Target");
        let dissolve_textures =
            h.create_dual_texture_bind_group(target_a_id, noise.handle, "TC30 Dual Textures");

        let mut chroma_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_a_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        chroma_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                pipe_chroma,
                DrawAction::Procedural {
                    vertex_count: operation_chroma["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
            .with_bind_group(1, sprite_bind_group, Vec::new())],
        );

        let clear = &graph_spec["passes"][1]["clear_color"];
        let mut dissolve_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color([
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ]);
        dissolve_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                pipe_dissolve,
                DrawAction::Procedural {
                    vertex_count: operation_dissolve["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, dissolve_textures, Vec::new())
            .with_bind_group(1, dissolve_bind_group, Vec::new())],
        );

        let cold_ms = execute_graph_pair(&mut h, &chroma_graph, &dissolve_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC30 cold readback failed");
        let warm_ms = execute_graph_pair(&mut h, &chroma_graph, &dissolve_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC30 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC30 output changed between runs"
        );

        let output_path = std::path::Path::new("tests/outputs/desktop/tc30_dissolve.png");
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            output_path,
        )
        .expect("TC30 PNG export failed");
        fs::write(
            "tests/outputs/desktop/tc30_dissolve_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC30",
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "2 pass (chroma key → dissolve/burn) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "manifest": "tests/shared_assets/manifests/tc30_dissolve.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "cold_render_time_ms": cold_ms,
            "warm_render_time_ms": warm_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
            "cache_output_equal": true,
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "instance_count": 2,
            "pass_count": graph_spec["passes"].as_array().unwrap().len()
        });
        fs::write(
            "tests/outputs/desktop/tc30_dissolve_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
