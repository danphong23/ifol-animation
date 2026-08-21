mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DirBlurUniform {
    angle: f32,
    strength: f32,
    samples: f32,
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

fn execute_pair(h: &mut DesktopTestHarness, first: &RenderGraph, second: &RenderGraph) -> f64 {
    let started = Instant::now();
    h.executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, first)
        .expect("TC34 chroma pass failed");
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, second)
        .expect("TC34 blur pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc34_directional_blur() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc34_directional_blur.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let graph = &manifest["graph"];
        let chroma = &graph["operations"][0];
        let blur = &graph["operations"][1];
        let width = graph["target"]["width"].as_u64().unwrap() as u32;
        let height = graph["target"]["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let chroma_pipeline = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let blur_pipeline = h.register_pipeline(
            "directional_blur.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let crop = &chroma["uniform"];
        let cw = (crop["uv_max"][0].as_f64().unwrap() - crop["uv_min"][0].as_f64().unwrap())
            * heroes.width as f64;
        let ch = (crop["uv_max"][1].as_f64().unwrap() - crop["uv_min"][1].as_f64().unwrap())
            * heroes.height as f64;
        let sy = crop["scale_y"].as_f64().unwrap();
        let sprite = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [
                (sy * cw / ch / (width as f64 / height as f64)) as f32,
                sy as f32,
            ],
            uv_min: [
                crop["uv_min"][0].as_f64().unwrap() as f32,
                crop["uv_min"][1].as_f64().unwrap() as f32,
            ],
            uv_max: [
                crop["uv_max"][0].as_f64().unwrap() as f32,
                crop["uv_max"][1].as_f64().unwrap() as f32,
            ],
            key_color: [0.0, 1.0, 0.0],
            tolerance: crop["tolerance"].as_f64().unwrap() as f32,
            smoothness: crop["smoothness"].as_f64().unwrap() as f32,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let sprite_bg = h.create_custom_uniform_bind_group(sprite, "TC34 Mage");
        let blur_bg = h.create_custom_uniform_bind_group(
            DirBlurUniform {
                angle: blur["uniform"]["angle"].as_f64().unwrap() as f32,
                strength: blur["uniform"]["strength"].as_f64().unwrap() as f32,
                samples: blur["uniform"]["samples"].as_f64().unwrap() as f32,
                _pad: 0.0,
            },
            "TC34 Blur",
        );
        let (chroma_id, _chroma_texture) = h.create_target("TC34 Chroma Target");
        let (final_id, final_texture) = h.create_target("TC34 Final Target");
        let blur_texture_bg = h.create_texture_bind_group(chroma_id, "TC34 Blur Texture");
        let mut chroma_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: chroma_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        chroma_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                chroma_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
                .with_bind_group(0, heroes.bind_group, Vec::new())
            .with_bind_group(1, sprite_bg, Vec::new())],
        );
        let clear = &graph["passes"][1]["clear_color"];
        let mut blur_graph = RenderGraph::new(RenderTarget::Offscreen {
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
        blur_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                blur_pipeline,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, blur_texture_bg, Vec::new())
            .with_bind_group(1, blur_bg, Vec::new())],
        );
        let cold_ms = execute_pair(&mut h, &chroma_graph, &blur_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC34 cold readback failed");
        let warm_ms = execute_pair(&mut h, &chroma_graph, &blur_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC34 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC34 output changed between runs"
        );
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            std::path::Path::new("tests/outputs/desktop/tc34_directional_blur.png"),
        )
        .expect("TC34 PNG export failed");
        fs::write(
            "tests/outputs/desktop/tc34_directional_blur_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({ "test_case": "TC34", "width": raw.width, "height": raw.height, "format": "Rgba8UnormSrgb", "adapter_name": h.engine.adapter_info().name, "backend": format!("{:?}", h.engine.adapter_info().backend), "device_type": format!("{:?}", h.engine.adapter_info().device_type), "timing_scope": "2 pass (chroma key → 30 degree directional blur) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback", "raw_fingerprint": fnv1a64(&raw.bytes), "manifest": "tests/shared_assets/manifests/tc34_directional_blur.json", "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()), "cold_render_time_ms": cold_ms, "warm_render_time_ms": warm_ms, "warm_iteration_count": 1, "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0, "cache_output_equal": true, "node_count": graph["node_count"], "draw_commands": graph["command_count"], "instance_count": 2, "pass_count": graph["passes"].as_array().unwrap().len() });
        fs::write(
            "tests/outputs/desktop/tc34_directional_blur_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
