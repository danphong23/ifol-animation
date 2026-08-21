mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LightSweepUniform {
    progress: f32,
    angle: f32,
    width: f32,
    intensity: f32,
    color: [f32; 3],
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
        .expect("TC31 chroma pass failed");
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, second)
        .expect("TC31 sweep pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

#[test]
fn run_tc31_light_sweep() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc31_light_sweep.json");
        let manifest: serde_json::Value = serde_json::from_str(manifest_text).unwrap();
        let graph = &manifest["graph"];
        let chroma = &graph["operations"][0];
        let sweep = &graph["operations"][1];
        let width = graph["target"]["width"].as_u64().unwrap() as u32;
        let height = graph["target"]["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let chroma_pipeline = h.register_pipeline(
            chroma["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let sweep_pipeline = h.register_pipeline(
            sweep["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let crop = &chroma["uniform"];
        let screen_aspect = width as f64 / height as f64;
        let crop_width = (crop["uv_max"][0].as_f64().unwrap()
            - crop["uv_min"][0].as_f64().unwrap())
            * heroes.width as f64;
        let crop_height = (crop["uv_max"][1].as_f64().unwrap()
            - crop["uv_min"][1].as_f64().unwrap())
            * heroes.height as f64;
        let scale_y = crop["scale_y"].as_f64().unwrap();
        let scale_x = (scale_y * crop_width / crop_height / screen_aspect) as f32;
        let sprite_uniform = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [scale_x, scale_y as f32],
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
        let sprite_bg = h.create_custom_uniform_bind_group(sprite_uniform, "TC31 Mage Chroma");
        let sweep_uniform = &sweep["uniform"];
        let sweep_bg = h.create_custom_uniform_bind_group(
            LightSweepUniform {
                progress: sweep_uniform["progress"].as_f64().unwrap() as f32,
                angle: sweep_uniform["angle"].as_f64().unwrap() as f32,
                width: sweep_uniform["width"].as_f64().unwrap() as f32,
                intensity: sweep_uniform["intensity"].as_f64().unwrap() as f32,
                color: [
                    sweep_uniform["color"][0].as_f64().unwrap() as f32,
                    sweep_uniform["color"][1].as_f64().unwrap() as f32,
                    sweep_uniform["color"][2].as_f64().unwrap() as f32,
                ],
                _pad: 0.0,
            },
            "TC31 Sweep Uniform",
        );
        let (chroma_id, _chroma_texture) = h.create_target("TC31 Chroma Target");
        let (final_id, final_texture) = h.create_target("TC31 Final Target");
        let sweep_texture_bg = h.create_texture_bind_group(chroma_id, "TC31 Sweep Texture");

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
                    vertex_count: chroma["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..1,
                },
            )
                .with_bind_group(0, heroes.bind_group, Vec::new())
            .with_bind_group(1, sprite_bg, Vec::new())],
        );
        let clear = &graph["passes"][1]["clear_color"];
        let mut sweep_graph = RenderGraph::new(RenderTarget::Offscreen {
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
        sweep_graph.add_batch(
            &mut h.pool,
            vec![DrawCommand::new(
                sweep_pipeline,
                DrawAction::Procedural {
                    vertex_count: sweep["vertex_count"].as_u64().unwrap() as u32,
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, sweep_texture_bg, Vec::new())
            .with_bind_group(1, sweep_bg, Vec::new())],
        );

        let cold_ms = execute_pair(&mut h, &chroma_graph, &sweep_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC31 cold readback failed");
        let warm_ms = execute_pair(&mut h, &chroma_graph, &sweep_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("TC31 warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "TC31 output changed between runs"
        );
        let output_path = std::path::Path::new("tests/outputs/desktop/tc31_light_sweep.png");
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            output_path,
        )
        .expect("TC31 PNG export failed");
        fs::write(
            "tests/outputs/desktop/tc31_light_sweep_desktop.bin",
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC31",
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "2 pass (chroma key → diagonal light sweep) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "manifest": "tests/shared_assets/manifests/tc31_light_sweep.json",
            "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
            "cold_render_time_ms": cold_ms,
            "warm_render_time_ms": warm_ms,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
            "cache_output_equal": true,
            "node_count": graph["node_count"],
            "draw_commands": graph["command_count"],
            "instance_count": 2,
            "pass_count": graph["passes"].as_array().unwrap().len()
        });
        fs::write(
            "tests/outputs/desktop/tc31_light_sweep_desktop.json",
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
