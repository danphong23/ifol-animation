#![allow(dead_code)]

use super::harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ExposureUniform {
    zebra_threshold: f32,
    zebra_speed: f32,
    time: f32,
    mode: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AtlasUniform {
    pos: [f32; 2],
    scale: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    texture_size: [f32; 2],
    enable_clamp: f32,
    tolerance: f32,
    smoothness: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    key_color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SoftParticleUniform {
    pos: [f32; 2],
    scale: [f32; 2],
    particle_depth: f32,
    softness: f32,
    core_intensity: f32,
    _pad: f32,
    particle_color: [f32; 4],
}

pub(crate) fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(crate) fn execute_graphs(
    h: &mut DesktopTestHarness,
    graphs: &[&RenderGraph],
    target: &wgpu::Texture,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let mut submission = None;
    for graph in graphs {
        submission = Some(
            h.executor
                .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
                .unwrap(),
        );
    }
    let submission = submission.expect("at least one graph is required");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let raw = h
        .engine
        .read_texture_to_raw_with_format_checked(target, wgpu::TextureFormat::Rgba8UnormSrgb)
        .unwrap();
    (elapsed, raw.bytes)
}

pub(crate) fn record(
    h: &mut DesktopTestHarness,
    graphs: &[&RenderGraph],
    target: &wgpu::Texture,
    output: &str,
    manifest_text: &str,
    manifest: &Value,
    timing_scope: &str,
) {
    let (cold_ms, cold_bytes) = execute_graphs(h, graphs, target);
    let (warm_ms, bytes) = execute_graphs(h, graphs, target);
    assert_eq!(
        cold_bytes, bytes,
        "{output} output changed between cold and warm runs"
    );

    let output_dir = std::path::Path::new("tests/outputs/desktop");
    fs::create_dir_all(output_dir).unwrap();
    h.save_texture_to_file_checked(
        target,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        output_dir.join(format!("{output}.png")),
    )
    .unwrap();
    fs::write(output_dir.join(format!("{output}_desktop.bin")), &bytes).unwrap();

    let graph = &manifest["graph"];
    let metadata = serde_json::json!({
        "test_case": manifest["test_case"],
        "width": graph["target"]["width"],
        "height": graph["target"]["height"],
        "format": "Rgba8UnormSrgb",
        "adapter_name": h.engine.adapter_info().name,
        "backend": format!("{:?}", h.engine.adapter_info().backend),
        "device_type": format!("{:?}", h.engine.adapter_info().device_type),
        "timing_scope": timing_scope,
        "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU",
        "raw_fingerprint": fnv1a64(&bytes),
        "manifest": format!("tests/shared_assets/manifests/{output}.json"),
        "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
        "cold_render_time_ms": cold_ms,
        "warm_render_time_ms": warm_ms,
        "warm_iteration_count": 1,
        "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
        "cache_output_equal": true,
        "validation_passed": true,
        "validation_error": Value::Null,
        "node_count": graph["node_count"],
        "draw_commands": graph["command_count"],
        "instance_count": graph["operations"].as_array().unwrap().len(),
        "pass_count": graph["passes"].as_array().unwrap().len()
    });
    fs::write(
        output_dir.join(format!("{output}_desktop.json")),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .unwrap();
}

fn sprite_uniform(
    pos: [f32; 2],
    scale: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    z_depth: f32,
) -> SpriteUniform {
    SpriteUniform {
        pos,
        scale,
        uv_min,
        uv_max,
        key_color: [0.0, 1.0, 0.0],
        tolerance: 0.48,
        smoothness: 0.10,
        z_depth,
        opacity: 1.0,
        _pad: 0.0,
    }
}

pub fn run_tc50() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc50_exposure_inspector.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let background = h.register_pipeline(
            "texture_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );
        let chroma = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let inspector = h.register_pipeline(
            "exposure_inspector.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let aspect = 800.0 / 600.0;
        let crop_w = (0.28 - 0.005) * heroes.width as f32;
        let crop_h = (0.98 - 0.01) * heroes.height as f32;
        let sprite = h.create_custom_uniform_bind_group(
            sprite_uniform(
                [0.0, 0.0],
                [0.8 * (crop_w / crop_h) / aspect, 0.8],
                [0.005, 0.01],
                [0.28, 0.98],
                0.5,
            ),
            "TC50 Paladin",
        );
        let exposure = h.create_custom_uniform_bind_group(
            ExposureUniform {
                zebra_threshold: 0.8,
                zebra_speed: 2.0,
                time: 1.0,
                mode: 0.0,
            },
            "TC50 Exposure",
        );
        let (scene_id, _) = h.create_target("TC50 Scene");
        let (final_id, final_texture) = h.create_target("TC50 Final");
        let scene_texture = h.create_texture_bind_group(scene_id, "TC50 Scene Texture");
        let mut scene_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        scene_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    background,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new()),
                DrawCommand::new(
                    chroma,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, sprite, Vec::new()),
            ],
        );
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        final_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    inspector,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scene_texture, Vec::new())
                .with_bind_group(1, exposure, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&scene_graph, &final_graph],
            &final_texture,
            "tc50_exposure_inspector",
            manifest_text,
            &manifest,
            "2 pass scene composition → exposure inspector + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}

pub fn run_tc51() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc51_atlas_clamp.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let background = h.register_pipeline(
            "texture_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );
        let atlas = h.register_pipeline(
            "atlas_clamp.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let aspect = 800.0 / 600.0;
        let make_uniform = |pos: [f32; 2], uv_min: [f32; 2], uv_max: [f32; 2]| AtlasUniform {
            pos,
            scale: [
                0.82 * ((uv_max[0] - uv_min[0]) * heroes.width as f32
                    / ((uv_max[1] - uv_min[1]) * heroes.height as f32))
                    / aspect,
                0.82,
            ],
            uv_min,
            uv_max,
            texture_size: [heroes.width as f32, heroes.height as f32],
            enable_clamp: 1.0,
            tolerance: 0.48,
            smoothness: 0.10,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
            key_color: [0.0, 1.0, 0.0, 1.0],
        };
        let paladin = h.create_custom_uniform_bind_group(
            make_uniform([-0.4, 0.0], [0.005, 0.01], [0.28, 0.98]),
            "TC51 Paladin",
        );
        let mage = h.create_custom_uniform_bind_group(
            make_uniform([0.4, 0.0], [0.27, 0.01], [0.54, 0.98]),
            "TC51 Mage",
        );
        let (target_id, target_texture) = h.create_target("TC51 Final");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    background,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new()),
                DrawCommand::new(
                    atlas,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, paladin, Vec::new()),
                DrawCommand::new(
                    atlas,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, mage, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_texture,
            "tc51_atlas_clamp",
            manifest_text,
            &manifest,
            "1 pass background + 2 atlas sprites + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}

pub fn run_tc52() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc52_soft_particles.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let alpha = wgpu::BlendState::ALPHA_BLENDING;
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let sprite = h.register_pipeline("chroma_key_cropped.wgsl", Some(alpha), true, true);
        let particle = h.register_pipeline("soft_particle.wgsl", Some(additive), true, true);
        let bg = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            0.95,
        ));
        let aspect = 800.0 / 600.0;
        let crop_w = (0.28 - 0.005) * heroes.width as f32;
        let crop_h = (0.98 - 0.01) * heroes.height as f32;
        let paladin = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, 0.0],
            [0.85 * (crop_w / crop_h) / aspect, 0.85],
            [0.005, 0.01],
            [0.28, 0.98],
            0.50,
        ));
        let particle_uniform = SoftParticleUniform {
            pos: [0.10, 0.05],
            scale: [0.38, 0.38 * aspect],
            particle_depth: 0.48,
            softness: 0.25,
            core_intensity: 2.5,
            _pad: 0.0,
            particle_color: [0.15, 0.85, 1.0, 0.90],
        };
        let particle_bg =
            h.create_custom_uniform_bind_group(particle_uniform, "TC52 Energy Sphere");
        let (target_id, target_texture) = h.create_target("TC52 Final");
        let (depth_id, _) = h.create_depth_target("TC52 Depth");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph.depth_stencil = Some(depth_id);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sprite,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new())
                .with_bind_group(1, bg, Vec::new()),
                DrawCommand::new(
                    sprite,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, paladin, Vec::new()),
                DrawCommand::new(
                    particle,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, particle_bg, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_texture,
            "tc52_soft_particles",
            manifest_text,
            &manifest,
            "1 pass depth-tested sprites + additive particle + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}
