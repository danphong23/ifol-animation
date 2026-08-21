use super::harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GenericUniform {
    data: [f32; 16],
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub enum Effect {
    Glassmorphism,
    SelectiveColor,
    MotionEcho,
    BokehDof,
    TrimPaths,
}

impl Effect {
    fn manifest(self) -> &'static str {
        match self {
            Self::Glassmorphism => {
                include_str!("../shared_assets/manifests/tc45_glassmorphism.json")
            }
            Self::SelectiveColor => {
                include_str!("../shared_assets/manifests/tc46_selective_color.json")
            }
            Self::MotionEcho => include_str!("../shared_assets/manifests/tc47_motion_echo.json"),
            Self::BokehDof => include_str!("../shared_assets/manifests/tc48_bokeh_dof.json"),
            Self::TrimPaths => include_str!("../shared_assets/manifests/tc49_trim_paths.json"),
        }
    }

    fn shader(self) -> &'static str {
        match self {
            Self::Glassmorphism => "glassmorphism.wgsl",
            Self::SelectiveColor => "selective_color.wgsl",
            Self::MotionEcho => "motion_echo.wgsl",
            Self::BokehDof => "bokeh_dof.wgsl",
            Self::TrimPaths => "trim_paths.wgsl",
        }
    }

    fn output(self) -> &'static str {
        match self {
            Self::Glassmorphism => "tc45_glassmorphism",
            Self::SelectiveColor => "tc46_selective_color",
            Self::MotionEcho => "tc47_motion_echo",
            Self::BokehDof => "tc48_bokeh_dof",
            Self::TrimPaths => "tc49_trim_paths",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Glassmorphism => "TC45 Glassmorphism",
            Self::SelectiveColor => "TC46 Selective Color",
            Self::MotionEcho => "TC47 Motion Echo",
            Self::BokehDof => "TC48 Bokeh DOF",
            Self::TrimPaths => "TC49 Trim Paths",
        }
    }

    fn uses_mage(self) -> bool {
        matches!(self, Self::MotionEcho | Self::TrimPaths)
    }

    fn chroma_only_final(self) -> bool {
        matches!(self, Self::MotionEcho)
    }

    fn uniform(self) -> [f32; 16] {
        match self {
            Self::Glassmorphism => [
                0.55, 0.5, 0.25, 0.28, 0.035, 3.5, 0.015, 0.005, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                0.0,
            ],
            Self::SelectiveColor => [
                0.95, 0.08, 0.05, 1.4, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            Self::MotionEcho => [
                -0.05, 0.0, 0.65, 1.2, 5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            Self::BokehDof => [
                0.5, 0.5, 0.22, 3.5, 6.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ],
            Self::TrimPaths => [
                0.5, 0.5, 0.18, 0.38, 0.04, 0.006, 0.6, 0.4, 2.5, 0.05, 0.9, 0.0, 0.1, 0.9, 1.0,
                1.0,
            ],
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

pub fn run(effect: Effect) {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async move {
        let manifest_text = effect.manifest();
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let graph = &manifest["graph"];
        let target = graph["target"].clone();
        let width = target["width"].as_u64().unwrap() as u32;
        let height = target["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let background_pipeline = h.register_pipeline(
            "texture_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );
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
        let (uv_min, uv_max, pos, scale_y) = if effect.uses_mage() {
            (
                [0.27, 0.01],
                [0.54, 0.98],
                if matches!(effect, Effect::MotionEcho) {
                    [0.15, 0.0]
                } else {
                    [0.0, 0.0]
                },
                if matches!(effect, Effect::MotionEcho | Effect::TrimPaths) {
                    0.75
                } else {
                    0.8
                },
            )
        } else {
            (
                [0.005, 0.01],
                [0.28, 0.98],
                if matches!(effect, Effect::Glassmorphism) {
                    [-0.35, 0.0]
                } else {
                    [0.0, 0.0]
                },
                if matches!(effect, Effect::Glassmorphism) {
                    0.7
                } else {
                    0.8
                },
            )
        };
        let crop_width = (uv_max[0] - uv_min[0]) * heroes.width as f32;
        let crop_height = (uv_max[1] - uv_min[1]) * heroes.height as f32;
        let sprite = super::harness::SpriteUniform {
            pos,
            scale: [
                scale_y * (crop_width / crop_height) / (width as f32 / height as f32),
                scale_y,
            ],
            uv_min,
            uv_max,
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.1,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let sprite_bg = h.create_custom_uniform_bind_group(sprite, effect.label());
        let effect_bg = h.create_custom_uniform_bind_group(
            GenericUniform {
                data: effect.uniform(),
            },
            effect.label(),
        );
        let (first_id, _) = h.create_target("First Target");
        let (final_id, final_texture) = h.create_target("Final Target");
        let first_texture_bg = h.create_texture_bind_group(first_id, "Effect Source");
        let mut first_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: first_id,
            width,
            height,
        })
        .with_clear_color([
            0.0,
            0.0,
            0.0,
            if effect.chroma_only_final() { 0.0 } else { 1.0 },
        ]);
        if effect.chroma_only_final() {
            first_graph.add_batch(
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
        } else {
            first_graph.add_batch(
                &mut h.pool,
                vec![
                    DrawCommand::new(
                        background_pipeline,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, scifi.bind_group, Vec::new()),
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
        }
        let clear = [0.0, 0.0, 0.0, 1.0];
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width,
            height,
        })
        .with_clear_color(clear);
        if effect.chroma_only_final() {
            final_graph.add_batch(
                &mut h.pool,
                vec![
                    DrawCommand::new(
                        background_pipeline,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, scifi.bind_group, Vec::new()),
                    DrawCommand::new(
                        effect_pipeline,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, first_texture_bg, Vec::new())
                    .with_bind_group(1, effect_bg, Vec::new()),
                ],
            );
        } else {
            final_graph.add_batch(
                &mut h.pool,
                vec![
                    DrawCommand::new(
                        effect_pipeline,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, first_texture_bg, Vec::new())
                    .with_bind_group(1, effect_bg, Vec::new()),
                ],
            );
        }
        let cold_ms = execute_pair(&mut h, &first_graph, &final_graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &final_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .unwrap();
        let warm_ms = execute_pair(&mut h, &first_graph, &final_graph);
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
        let output = effect.output();
        h.save_texture_to_file_checked(
            &final_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            std::path::Path::new(&format!("tests/outputs/desktop/{output}.png")),
        )
        .unwrap();
        fs::write(
            format!("tests/outputs/desktop/{output}_desktop.bin"),
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({ "test_case": manifest["test_case"], "width": raw.width, "height": raw.height, "format": "Rgba8UnormSrgb", "adapter_name": h.engine.adapter_info().name, "backend": format!("{:?}", h.engine.adapter_info().backend), "device_type": format!("{:?}", h.engine.adapter_info().device_type), "timing_scope": "2 pass scene/effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback", "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU", "raw_fingerprint": fnv1a64(&raw.bytes), "manifest": format!("tests/shared_assets/manifests/{output}.json"), "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()), "cold_render_time_ms": cold_ms, "warm_render_time_ms": warm_ms, "warm_iteration_count": 1, "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0, "cache_output_equal": true, "node_count": graph["node_count"], "draw_commands": graph["command_count"], "instance_count": graph["operations"].as_array().unwrap().len(), "pass_count": graph["passes"].as_array().unwrap().len() });
        fs::write(
            format!("tests/outputs/desktop/{output}_desktop.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();
    });
}
