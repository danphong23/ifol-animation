#![allow(dead_code)]

use super::advanced_effects::{fnv1a64, record};
use super::harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::BindGroupHandle;
use serde_json::Value;
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn sprite_uniform(
    pos: [f32; 2],
    scale: [f32; 2],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    key_color: [f32; 3],
    tolerance: f32,
    smoothness: f32,
    z_depth: f32,
    opacity: f32,
) -> SpriteUniform {
    SpriteUniform {
        pos,
        scale,
        uv_min,
        uv_max,
        key_color,
        tolerance,
        smoothness,
        z_depth,
        opacity,
        _pad: 0.0,
    }
}

pub fn run_tc56() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc56_dynamic_resize.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let city = h.load_texture_exact("canonical_bg_anime_city.png");
        let sprite = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let blit = h.register_pipeline(
            "sprite_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let (left_id, left_texture) = h.create_custom_target(400, 600, "TC56 Left");
        let (right_id, right_texture) = h.create_custom_target(400, 600, "TC56 Right");
        let (final_id, final_texture) = h.create_target("TC56 Final");

        let left_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, -0.1],
            [
                0.85 * ((0.52 - 0.30) * heroes.width as f32 / heroes.height as f32)
                    / (400.0 / 600.0),
                0.85,
            ],
            [0.30, 0.0],
            [0.52, 1.0],
            heroes.background_key_color,
            0.45,
            0.12,
            0.5,
            1.0,
        ));
        let right_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, -0.1],
            [
                0.85 * ((0.28 - 0.0) * heroes.width as f32 / heroes.height as f32)
                    / (400.0 / 600.0),
                0.85,
            ],
            [0.0, 0.0],
            [0.28, 1.0],
            heroes.background_key_color,
            0.45,
            0.12,
            0.5,
            1.0,
        ));
        let bg_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            city.background_key_color,
            0.0,
            0.0,
            0.9,
            0.35,
        ));
        let left_panel = h.create_sprite_uniform_bind_group(sprite_uniform(
            [-0.5, 0.0],
            [0.46, 0.92],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.0,
            0.0,
            0.5,
            1.0,
        ));
        let right_panel = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.5, 0.0],
            [0.46, 0.92],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.0,
            0.0,
            0.5,
            1.0,
        ));
        let left_bg = h.create_texture_bind_group(left_id, "TC56 Left Source");
        let right_bg = h.create_texture_bind_group(right_id, "TC56 Right Source");

        let mut left = RenderGraph::new(RenderTarget::Offscreen {
            color: left_id,
            width: 400,
            height: 600,
        })
        .with_clear_color([0.1, 0.08, 0.15, 1.0]);
        left.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sprite,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, left_uniform, Vec::new()),
            ],
        );

        let mut right = RenderGraph::new(RenderTarget::Offscreen {
            color: right_id,
            width: 400,
            height: 600,
        })
        .with_clear_color([0.08, 0.12, 0.15, 1.0]);
        right.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sprite,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, right_uniform, Vec::new()),
            ],
        );

        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.04, 0.04, 0.06, 1.0]);
        final_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    sprite,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, city.bind_group, Vec::new())
                .with_bind_group(1, bg_uniform, Vec::new()),
                DrawCommand::new(
                    blit,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, left_bg, Vec::new())
                .with_bind_group(1, left_panel, Vec::new()),
                DrawCommand::new(
                    blit,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, right_bg, Vec::new())
                .with_bind_group(1, right_panel, Vec::new()),
            ],
        );

        record(
            &mut h,
            &[&left, &right, &final_graph],
            &final_texture,
            "tc56_dynamic_resize",
            manifest_text,
            &manifest,
            "3 pass 400x600 left/right targets → 800x600 composition + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
        let _ = (left_texture, right_texture);
    });
}

pub fn run_tc57() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc57_stencil_mask.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let night = h.load_texture_exact("canonical_bg_nightsky.png");
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let mask = h.register_stencil_pipeline(
            "stencil_mask.wgsl",
            wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::IncrementClamp,
                },
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: !0,
                write_mask: !0,
            },
            wgpu::ColorWrites::empty(),
        );
        let content = h.register_stencil_pipeline(
            "chroma_key_cropped.wgsl",
            wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: !0,
                write_mask: 0,
            },
            wgpu::ColorWrites::ALL,
        );
        let night_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            night.background_key_color,
            0.0,
            0.0,
            0.5,
            1.0,
        ));
        let wizard_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.0, -0.05],
            [
                0.65 * ((0.52 - 0.30) * heroes.width as f32 / heroes.height as f32)
                    / (800.0 / 600.0),
                0.65,
            ],
            [0.30, 0.0],
            [0.52, 1.0],
            heroes.background_key_color,
            0.45,
            0.12,
            0.4,
            1.0,
        ));
        let (target_id, target_texture) = h.create_target("TC57 Final");
        let (depth_id, depth_texture) = h.create_depth_stencil_target("TC57 Depth Stencil");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.08, 1.0]);
        graph.depth_stencil = Some(depth_id);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    mask,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, night.bind_group, Vec::new())
                .with_bind_group(1, night_uniform, Vec::new()),
            ],
        );
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    content,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, night.bind_group, Vec::new())
                .with_bind_group(1, night_uniform, Vec::new()),
                DrawCommand::new(
                    content,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group, Vec::new())
                .with_bind_group(1, wizard_uniform, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_texture,
            "tc57_stencil_mask",
            manifest_text,
            &manifest,
            "1 pass stencil mask + masked background/wizard + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
        let _ = depth_texture;
    });
}

fn execute_mrt_and_graph(
    h: &mut DesktopTestHarness,
    mrt_pipeline: &wgpu::RenderPipeline,
    source_handle: BindGroupHandle,
    albedo_view: &wgpu::TextureView,
    emissive_view: &wgpu::TextureView,
    graph: &RenderGraph,
    final_texture: &wgpu::Texture,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let mrt_submission = {
        let source = h.registry.bind_group(&source_handle).unwrap();
        let mut encoder =
            h.engine
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("TC58 MRT Encoder"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("TC58 MRT Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: albedo_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: emissive_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    }),
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(mrt_pipeline);
            pass.set_bind_group(0, source, &[]);
            pass.draw(0..6, 0..1);
        }
        h.engine.queue().submit(Some(encoder.finish()))
    };
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(mrt_submission),
        timeout: None,
    });
    let composite_submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, &mut h.pool, graph)
        .unwrap();
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(composite_submission),
        timeout: None,
    });
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let raw = h
        .engine
        .read_texture_to_raw_with_format_checked(final_texture, wgpu::TextureFormat::Rgba8UnormSrgb)
        .unwrap();
    (elapsed, raw.bytes)
}

fn record_mrt(
    h: &mut DesktopTestHarness,
    mrt_pipeline: &wgpu::RenderPipeline,
    source_handle: BindGroupHandle,
    albedo_view: &wgpu::TextureView,
    emissive_view: &wgpu::TextureView,
    graph: &RenderGraph,
    final_texture: &wgpu::Texture,
    output: &str,
    manifest_text: &str,
    manifest: &Value,
) {
    let (cold_ms, cold_bytes) = execute_mrt_and_graph(
        h,
        mrt_pipeline,
        source_handle,
        albedo_view,
        emissive_view,
        graph,
        final_texture,
    );
    let (warm_ms, bytes) = execute_mrt_and_graph(
        h,
        mrt_pipeline,
        source_handle,
        albedo_view,
        emissive_view,
        graph,
        final_texture,
    );
    assert_eq!(
        cold_bytes, bytes,
        "{output} output changed between cold and warm runs"
    );
    let output_dir = Path::new("tests/outputs/desktop");
    fs::create_dir_all(output_dir).unwrap();
    h.save_texture_to_file_checked(
        final_texture,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        output_dir.join(format!("{output}.png")),
    )
    .unwrap();
    fs::write(output_dir.join(format!("{output}_desktop.bin")), &bytes).unwrap();
    let graph_spec = &manifest["graph"];
    let metadata = serde_json::json!({
        "test_case": manifest["test_case"],
        "width": graph_spec["target"]["width"],
        "height": graph_spec["target"]["height"],
        "format": "Rgba8UnormSrgb",
        "adapter_name": h.engine.adapter_info().name,
        "backend": format!("{:?}", h.engine.adapter_info().backend),
        "device_type": format!("{:?}", h.engine.adapter_info().device_type),
        "timing_scope": "1 pass 2-attachment MRT + 1 pass side-by-side composite + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
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
}

pub fn run_tc58() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc58_mrt_gbuffer.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let shader_code =
            fs::read_to_string(Path::new("tests/shared_assets/shaders/mrt_gbuffer.wgsl")).unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("TC58 MRT Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
            });
        let layout = h
            .engine
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TC58 MRT Layout"),
                bind_group_layouts: &[Some(&h.texture_bg_layout)],
                immediate_size: 0,
            });
        let mrt_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("TC58 MRT Pipeline"),
                    layout: Some(&layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[
                            Some(wgpu::ColorTargetState {
                                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                            Some(wgpu::ColorTargetState {
                                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                                blend: Some(wgpu::BlendState::REPLACE),
                                write_mask: wgpu::ColorWrites::ALL,
                            }),
                        ],
                        compilation_options: Default::default(),
                    }),
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let (albedo_id, albedo_texture) = h.create_target("TC58 Albedo");
        let (emissive_id, emissive_texture) = h.create_target("TC58 Emissive");
        let (final_id, final_texture) = h.create_target("TC58 Final");
        let albedo_view = albedo_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let emissive_view = emissive_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let blit = h.register_pipeline(
            "sprite_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let left_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [-0.5, 0.0],
            [0.48, 0.95],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.0,
            0.0,
            0.5,
            1.0,
        ));
        let right_uniform = h.create_sprite_uniform_bind_group(sprite_uniform(
            [0.5, 0.0],
            [0.48, 0.95],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0, 0.0],
            0.0,
            0.0,
            0.5,
            1.0,
        ));
        let albedo_bg = h.create_texture_bind_group(albedo_id, "TC58 Albedo View");
        let emissive_bg = h.create_texture_bind_group(emissive_id, "TC58 Emissive View");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.08, 1.0]);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    blit,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, albedo_bg, Vec::new())
                .with_bind_group(1, left_uniform, Vec::new()),
                DrawCommand::new(
                    blit,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, emissive_bg, Vec::new())
                .with_bind_group(1, right_uniform, Vec::new()),
            ],
        );
        record_mrt(
            &mut h,
            &mrt_pipeline,
            heroes.bind_group,
            &albedo_view,
            &emissive_view,
            &graph,
            &final_texture,
            "tc58_mrt_gbuffer",
            manifest_text,
            &manifest,
        );
    });
}
