#![allow(dead_code)]

use super::advanced_effects::record;
use super::harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::BindGroupHandle;
use serde_json::Value;

fn manifest(name: &str) -> (&'static str, Value) {
    let text = match name {
        "tc59_sampler_modes" => include_str!("../shared_assets/manifests/tc59_sampler_modes.json"),
        "tc60_ping_pong" => include_str!("../shared_assets/manifests/tc60_ping_pong.json"),
        _ => unreachable!("unsupported manifest"),
    };
    (text, serde_json::from_str(text).unwrap())
}

fn sampler_bind_group(
    h: &mut DesktopTestHarness,
    texture: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    label: &str,
    id: u64,
) -> BindGroupHandle {
    let bind_group = h
        .engine
        .device()
        .create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &h.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(texture),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: Some(label),
        });
    let handle = BindGroupHandle(id);
    h.registry
        .insert_bind_group_with_descriptor(
            handle,
            bind_group,
            ifol_gpu::resources::BindGroupResourceDescriptor {
                dynamic_offset_count: 0,
                dynamic_offset_alignment: 0,
                layout_signature: 1,
            },
        )
        .unwrap();
    handle
}

pub fn run_tc59() {
    pollster::block_on(async {
        let (manifest_text, manifest_value) = manifest("tc59_sampler_modes");
        let mut h = DesktopTestHarness::new(800, 600).await;
        let source = h.load_texture_exact("props_characters.jpg");
        let pipeline = h.register_pipeline(
            "sampler_modes.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let repeat = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let mirror = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let clamp = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let view = h.registry.texture(&source.handle).unwrap().0.clone();
        let bg_repeat = sampler_bind_group(&mut h, &view, &repeat, "TC59 Repeat", 1000);
        let bg_mirror = sampler_bind_group(&mut h, &view, &mirror, "TC59 MirrorRepeat", 1001);
        let bg_clamp = sampler_bind_group(&mut h, &view, &clamp, "TC59 ClampToEdge", 1002);
        let mut uniform = |pos: [f32; 2], label: &str| {
            h.create_custom_uniform_bind_group(
                SpriteUniform {
                    pos,
                    scale: [0.26, 0.35],
                    uv_min: [-0.5, -0.5],
                    uv_max: [1.5, 1.5],
                    key_color: [0.0, 0.0, 0.0],
                    tolerance: 0.0,
                    smoothness: 0.0,
                    z_depth: 0.5,
                    opacity: 1.0,
                    _pad: 0.0,
                },
                label,
            )
        };
        let u_repeat = uniform([-0.62, 0.0], "TC59 Repeat Uniform");
        let u_mirror = uniform([0.0, 0.0], "TC59 Mirror Uniform");
        let u_clamp = uniform([0.62, 0.0], "TC59 Clamp Uniform");
        let (target, target_tex) = h.create_target("TC59 Final");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.06, 0.06, 0.09, 1.0]);
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
                .with_bind_group(0, bg_repeat, Vec::new())
                .with_bind_group(1, u_repeat, Vec::new()),
                DrawCommand::new(
                    pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, bg_mirror, Vec::new())
                .with_bind_group(1, u_mirror, Vec::new()),
                DrawCommand::new(
                    pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, bg_clamp, Vec::new())
                .with_bind_group(1, u_clamp, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_tex,
            "tc59_sampler_modes",
            manifest_text,
            &manifest_value,
            "1 pass, 3 sampler address modes + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}

pub fn run_tc60() {
    pollster::block_on(async {
        let (manifest_text, manifest_value) = manifest("tc60_ping_pong");
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let sprite = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let blit = h.register_pipeline(
            "ping_pong_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let (ping, _ping_tex) = h.create_target("TC60 Ping");
        let (pong, _pong_tex) = h.create_target("TC60 Pong");
        let (final_target, final_tex) = h.create_target("TC60 Final");
        let bg_ping = h.create_texture_bind_group(ping, "TC60 Ping Source");
        let bg_pong = h.create_texture_bind_group(pong, "TC60 Pong Source");
        let wizard = h.build_sprite_uniform(
            &heroes,
            [0.0, 0.0],
            0.65,
            [0.30, 0.0],
            [0.52, 1.0],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let wizard_bg = h.create_sprite_uniform_bind_group(wizard);
        let zoom_out = h.create_custom_uniform_bind_group(
            SpriteUniform {
                pos: [0.008, 0.008],
                scale: [1.025, 1.025],
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                key_color: [0.0, 0.0, 0.0],
                tolerance: 0.0,
                smoothness: 0.0,
                z_depth: 0.5,
                opacity: 0.85,
                _pad: 0.0,
            },
            "TC60 Zoom Out",
        );
        let zoom_in = h.create_custom_uniform_bind_group(
            SpriteUniform {
                pos: [-0.005, -0.005],
                scale: [1.025, 1.025],
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                key_color: [0.0, 0.0, 0.0],
                tolerance: 0.0,
                smoothness: 0.0,
                z_depth: 0.5,
                opacity: 0.85,
                _pad: 0.0,
            },
            "TC60 Zoom In",
        );
        let copy = h.create_custom_uniform_bind_group(
            SpriteUniform {
                pos: [0.0, 0.0],
                scale: [1.0, 1.0],
                uv_min: [0.0, 0.0],
                uv_max: [1.0, 1.0],
                key_color: [0.0, 0.0, 0.0],
                tolerance: 0.0,
                smoothness: 0.0,
                z_depth: 0.5,
                opacity: 1.0,
                _pad: 0.0,
            },
            "TC60 Final Copy",
        );
        let mut graphs = Vec::new();
        let mut seed = RenderGraph::new(RenderTarget::Offscreen {
            color: ping,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.08, 1.0]);
        seed.add_batch(
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
                .with_bind_group(1, wizard_bg, Vec::new()),
            ],
        );
        graphs.push(seed);
        for _ in 0..8 {
            let mut to_pong = RenderGraph::new(RenderTarget::Offscreen {
                color: pong,
                width: 800,
                height: 600,
            });
            if graphs.len() == 1 {
                to_pong = to_pong.with_clear_color([0.0, 0.0, 0.0, 0.0]);
            }
            if graphs.len() == 1 {
                to_pong = to_pong.with_clear_color([0.0, 0.0, 0.0, 0.0]);
            }
            to_pong.add_batch(
                &mut h.pool,
                vec![
                    DrawCommand::new(
                        blit,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, bg_ping, Vec::new())
                    .with_bind_group(1, zoom_out, Vec::new()),
                ],
            );
            graphs.push(to_pong);
            let mut to_ping = RenderGraph::new(RenderTarget::Offscreen {
                color: ping,
                width: 800,
                height: 600,
            });
            to_ping.add_batch(
                &mut h.pool,
                vec![
                    DrawCommand::new(
                        blit,
                        DrawAction::Procedural {
                            vertex_count: 6,
                            instance_range: 0..1,
                        },
                    )
                    .with_bind_group(0, bg_pong, Vec::new())
                    .with_bind_group(1, zoom_in, Vec::new()),
                ],
            );
            graphs.push(to_ping);
        }
        let mut final_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.02, 0.04, 1.0]);
        final_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    blit,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, bg_ping, Vec::new())
                .with_bind_group(1, copy, Vec::new()),
            ],
        );
        graphs.push(final_graph);
        let refs: Vec<&RenderGraph> = graphs.iter().collect();
        record(
            &mut h,
            &refs,
            &final_tex,
            "tc60_ping_pong",
            manifest_text,
            &manifest_value,
            "18 graph executions (seed + 16 feedback + final copy) + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}
