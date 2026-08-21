use super::harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TrackMatteUniform {
    data: [f32; 4],
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
        .unwrap();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, second)
        .unwrap();
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

pub fn run() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc43_track_matte.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let graph_spec = &manifest["graph"];
        let width = graph_spec["target"]["width"].as_u64().unwrap() as u32;
        let height = graph_spec["target"]["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let chroma = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let matte = h.register_dual_texture_pipeline(
            "track_matte.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );

        let screen_aspect = width as f32 / height as f32;
        let uv_min = [0.005, 0.01];
        let uv_max = [0.28, 0.98];
        let crop_width = (uv_max[0] - uv_min[0]) * heroes.width as f32;
        let crop_height = (uv_max[1] - uv_min[1]) * heroes.height as f32;
        let sprite = SpriteUniform {
            pos: [0.0, 0.0],
            scale: [0.85 * (crop_width / crop_height) / screen_aspect, 0.85],
            uv_min,
            uv_max,
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.1,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let sprite_bg = h.create_custom_uniform_bind_group(sprite, "TC43 Paladin");
        let matte_bg = h.create_custom_uniform_bind_group(
            TrackMatteUniform {
                data: [0.0, 1.0, 0.0, 0.0],
            },
            "TC43 Track Matte",
        );
        let (matte_id, _) = h.create_target("Matte Target");
        let (final_id, final_texture) = h.create_target("Final Target");
        let dual_bg = h.create_dual_texture_bind_group(scifi.handle, matte_id, "SciFi + Matte");

        let mut matte_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: matte_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        matte_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    chroma,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, sprite_bg, Vec::new()),
            ],
        );

        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color([0.08, 0.08, 0.12, 1.0]);
        final_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    matte,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, dual_bg, Vec::new())
                .with_bind_group(1, matte_bg, Vec::new()),
            ],
        );

        let cold_ms = execute_pair(&mut h, &matte_graph, &final_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .unwrap();
        let warm_ms = execute_pair(&mut h, &matte_graph, &final_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .unwrap();
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "output changed between cold and warm runs"
        );

        let output = "tc43_track_matte";
        let output_dir = std::path::Path::new("tests/outputs/desktop");
        fs::create_dir_all(output_dir).unwrap();
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            output_dir.join(format!("{output}.png")),
        )
        .unwrap();
        fs::write(output_dir.join(format!("{output}_desktop.bin")), &raw.bytes).unwrap();
        let metadata = serde_json::json!({
            "test_case": manifest["test_case"],
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "2 pass (chroma matte → track matte) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU",
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "manifest": format!("tests/shared_assets/manifests/{output}.json"),
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "cold_render_time_ms": cold_ms,
            "warm_render_time_ms": warm_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
            "cache_output_equal": true,
            "node_count": graph_spec["node_count"],
            "draw_commands": graph_spec["command_count"],
            "instance_count": graph_spec["operations"].as_array().unwrap().len(),
            "pass_count": graph_spec["passes"].as_array().unwrap().len()
        });
        fs::write(
            output_dir.join(format!("{output}_desktop.json")),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
