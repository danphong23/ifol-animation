use super::harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ChromaticAberrationUniform {
    center: [f32; 2],
    amount: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct KaleidoscopeUniform {
    segments: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ScanlineUniform {
    lines_count: f32,
    speed: f32,
    time: f32,
    opacity: f32,
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VignetteUniform {
    vignette_radius: f32,
    vignette_softness: f32,
    grain_strength: f32,
    time: f32,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Effect {
    ChromaticAberration,
    Kaleidoscope,
    Scanlines,
    VignetteGrain,
}

impl Effect {
    fn manifest(self) -> &'static str {
        match self {
            Self::ChromaticAberration => {
                include_str!("../shared_assets/manifests/tc37_chromatic_aberration.json")
            }
            Self::Kaleidoscope => include_str!("../shared_assets/manifests/tc38_kaleidoscope.json"),
            Self::Scanlines => include_str!("../shared_assets/manifests/tc39_scanlines.json"),
            Self::VignetteGrain => {
                include_str!("../shared_assets/manifests/tc40_vignette_grain.json")
            }
        }
    }

    fn shader(self) -> &'static str {
        match self {
            Self::ChromaticAberration => "chromatic_aberration.wgsl",
            Self::Kaleidoscope => "kaleidoscope.wgsl",
            Self::Scanlines => "scanlines.wgsl",
            Self::VignetteGrain => "vignette_grain.wgsl",
        }
    }

    fn output(self) -> &'static str {
        match self {
            Self::ChromaticAberration => "tc37_chromatic_aberration",
            Self::Kaleidoscope => "tc38_kaleidoscope",
            Self::Scanlines => "tc39_scanlines",
            Self::VignetteGrain => "tc40_vignette_grain",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::ChromaticAberration => "TC37 Chromatic Aberration",
            Self::Kaleidoscope => "TC38 Kaleidoscope",
            Self::Scanlines => "TC39 Hologram Scanlines",
            Self::VignetteGrain => "TC40 Vignette Grain",
        }
    }
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
        .expect("first parity pass failed");
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, second)
        .expect("second parity pass failed");
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    started.elapsed().as_secs_f64() * 1000.0
}

pub fn run(effect: Effect) {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async move {
        let manifest_text = effect.manifest();
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let graph = &manifest["graph"];
        let chroma = &graph["operations"][0];
        let effect_operation = &graph["operations"][1];
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
        let effect_pipeline = h.register_pipeline(
            effect.shader(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let crop = &chroma["uniform"];
        let crop_width = (crop["uv_max"][0].as_f64().unwrap()
            - crop["uv_min"][0].as_f64().unwrap())
            * heroes.width as f64;
        let crop_height = (crop["uv_max"][1].as_f64().unwrap()
            - crop["uv_min"][1].as_f64().unwrap())
            * heroes.height as f64;
        let scale_y = crop["scale_y"].as_f64().unwrap();
        let sprite = super::harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [
                (scale_y * crop_width / crop_height / (width as f64 / height as f64)) as f32,
                scale_y as f32,
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
        let sprite_bg = h.create_custom_uniform_bind_group(sprite, effect.label());
        let effect_bg = match effect {
            Effect::ChromaticAberration => {
                let u = &effect_operation["uniform"];
                h.create_custom_uniform_bind_group(
                    ChromaticAberrationUniform {
                        center: [
                            u["center"][0].as_f64().unwrap() as f32,
                            u["center"][1].as_f64().unwrap() as f32,
                        ],
                        amount: u["amount"].as_f64().unwrap() as f32,
                        _pad: 0.0,
                    },
                    effect.label(),
                )
            }
            Effect::Kaleidoscope => h.create_custom_uniform_bind_group(
                KaleidoscopeUniform {
                    segments: effect_operation["uniform"]["segments"].as_f64().unwrap() as f32,
                    _pad0: 0.0,
                    _pad1: 0.0,
                    _pad2: 0.0,
                },
                effect.label(),
            ),
            Effect::Scanlines => {
                let u = &effect_operation["uniform"];
                h.create_custom_uniform_bind_group(
                    ScanlineUniform {
                        lines_count: u["lines_count"].as_f64().unwrap() as f32,
                        speed: u["speed"].as_f64().unwrap() as f32,
                        time: u["time"].as_f64().unwrap() as f32,
                        opacity: u["opacity"].as_f64().unwrap() as f32,
                        color: [
                            u["color"][0].as_f64().unwrap() as f32,
                            u["color"][1].as_f64().unwrap() as f32,
                            u["color"][2].as_f64().unwrap() as f32,
                            u["color"][3].as_f64().unwrap() as f32,
                        ],
                    },
                    effect.label(),
                )
            }
            Effect::VignetteGrain => {
                let u = &effect_operation["uniform"];
                h.create_custom_uniform_bind_group(
                    VignetteUniform {
                        vignette_radius: u["vignette_radius"].as_f64().unwrap() as f32,
                        vignette_softness: u["vignette_softness"].as_f64().unwrap() as f32,
                        grain_strength: u["grain_strength"].as_f64().unwrap() as f32,
                        time: u["time"].as_f64().unwrap() as f32,
                    },
                    effect.label(),
                )
            }
        };
        let (chroma_id, _) = h.create_target("Chroma Target");
        let (final_id, final_texture) = h.create_target("Final Target");
        let effect_texture_bg = h.create_texture_bind_group(chroma_id, "Effect Texture");
        let mut chroma_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: chroma_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        chroma_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    chroma_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, sprite_bg, Vec::new()),
            ],
        );
        let clear = &graph["passes"][1]["clear_color"];
        let mut effect_graph = RenderGraph::new(RenderTarget::Offscreen {
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
        effect_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    effect_pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, effect_texture_bg, Vec::new())
                .with_bind_group(1, effect_bg, Vec::new()),
            ],
        );
        let cold_ms = execute_pair(&mut h, &chroma_graph, &effect_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("cold readback failed");
        let warm_ms = execute_pair(&mut h, &chroma_graph, &effect_graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("warm readback failed");
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "output changed between cold and warm runs"
        );
        let output = effect.output();
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            std::path::Path::new(&format!("tests/outputs/desktop/{output}.png")),
        )
        .expect("PNG export failed");
        fs::write(
            format!("tests/outputs/desktop/{output}_desktop.bin"),
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({ "test_case": manifest["test_case"], "width": raw.width, "height": raw.height, "format": "Rgba8UnormSrgb", "adapter_name": h.engine.adapter_info().name, "backend": format!("{:?}", h.engine.adapter_info().backend), "device_type": format!("{:?}", h.engine.adapter_info().device_type), "timing_scope": "2 pass (chroma key → effect) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback", "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU", "raw_fingerprint": fnv1a64(&raw.bytes), "manifest": format!("tests/shared_assets/manifests/{output}.json"), "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()), "cold_render_time_ms": cold_ms, "warm_render_time_ms": warm_ms, "warm_iteration_count": 1, "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0, "cache_output_equal": true, "node_count": graph["node_count"], "draw_commands": graph["command_count"], "instance_count": 2, "pass_count": graph["passes"].as_array().unwrap().len() });
        fs::write(
            format!("tests/outputs/desktop/{output}_desktop.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
