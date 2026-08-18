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
    AspectFill,
    AnamorphicFlare,
}

impl Effect {
    fn manifest(self) -> &'static str {
        match self {
            Self::AspectFill => include_str!("../shared_assets/manifests/tc41_aspect_fill.json"),
            Self::AnamorphicFlare => {
                include_str!("../shared_assets/manifests/tc44_anamorphic_flare.json")
            }
        }
    }

    fn shader(self) -> &'static str {
        match self {
            Self::AspectFill => "aspect_fill.wgsl",
            Self::AnamorphicFlare => "anamorphic_flare.wgsl",
        }
    }

    fn output(self) -> &'static str {
        match self {
            Self::AspectFill => "tc41_aspect_fill",
            Self::AnamorphicFlare => "tc44_anamorphic_flare",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::AspectFill => "TC41 Aspect Fill",
            Self::AnamorphicFlare => "TC44 Anamorphic Flare",
        }
    }

    fn dimensions(self, graph: &Value) -> (u32, u32) {
        (
            graph["target"]["width"].as_u64().unwrap() as u32,
            graph["target"]["height"].as_u64().unwrap() as u32,
        )
    }

    fn uniform(self) -> [f32; 16] {
        match self {
            Self::AspectFill => [
                450.0 / 800.0,
                1376.0 / 768.0,
                2.5,
                0.6,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
            Self::AnamorphicFlare => [
                0.35, 3.5, 2.2, 0.0, 0.15, 0.65, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
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

fn execute(h: &mut DesktopTestHarness, graph: &RenderGraph) -> f64 {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
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
        let graph_spec = &manifest["graph"];
        let (width, height) = effect.dimensions(graph_spec);
        let mut h = DesktopTestHarness::new(width, height).await;
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let pipeline = h.register_pipeline(
            effect.shader(),
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let uniform = h.create_custom_uniform_bind_group(
            GenericUniform {
                data: effect.uniform(),
            },
            effect.label(),
        );
        let (target_id, target_texture) = h.create_target("Final Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new())
                .with_bind_group(1, uniform, Vec::new()),
            ],
        );

        let cold_ms = execute(&mut h, &graph);
        let cold_raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .unwrap();
        let warm_ms = execute(&mut h, &graph);
        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                &target_texture,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .unwrap();
        assert_eq!(
            cold_raw.bytes, raw.bytes,
            "output changed between cold and warm runs"
        );

        let output = effect.output();
        let output_dir = std::path::Path::new("tests/outputs/desktop");
        fs::create_dir_all(output_dir).unwrap();
        h.save_texture_to_file_checked(
            &target_texture,
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
            "timing_scope": "1 pass effect + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
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
