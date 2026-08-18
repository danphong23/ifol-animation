#![allow(dead_code)]

use super::advanced_effects::fnv1a64;
use super::harness::DesktopTestHarness;
use bytemuck::{Pod, Zeroable};
use ifol_gpu::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use serde_json::Value;
use std::fs;
use std::time::Instant;

fn parse_manifest(text: &'static str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn execute_graphs(
    h: &mut DesktopTestHarness,
    pool: &mut RenderNodePool,
    graphs: &[&RenderGraph],
    target: &wgpu::Texture,
    format: wgpu::TextureFormat,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let mut submission = None;
    for graph in graphs {
        submission = Some(
            h.executor
                .execute_checked(&h.engine, &h.registry, pool, graph)
                .unwrap(),
        );
    }
    let submission = submission.unwrap();
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let raw = h
        .engine
        .read_texture_to_raw_with_format_checked(target, format)
        .unwrap();
    (elapsed, raw.bytes)
}

fn write_result(
    h: &DesktopTestHarness,
    output: &str,
    manifest_text: &str,
    manifest: &Value,
    target: &wgpu::Texture,
    format: wgpu::TextureFormat,
    cold_ms: f64,
    cold_bytes: &[u8],
    warm_ms: f64,
    bytes: &[u8],
) {
    assert_eq!(
        cold_bytes, bytes,
        "{output} output changed between cold and warm runs"
    );
    let output_dir = std::path::Path::new("tests/outputs/desktop");
    fs::create_dir_all(output_dir).unwrap();
    h.save_texture_to_file_checked(target, format, output_dir.join(format!("{output}.png")))
        .unwrap();
    fs::write(output_dir.join(format!("{output}_desktop.bin")), bytes).unwrap();
    let graph = &manifest["graph"];
    let metadata = serde_json::json!({
        "test_case": manifest["test_case"],
        "width": graph["target"]["width"],
        "height": graph["target"]["height"],
        "format": format!("{format:?}"),
        "adapter_name": h.engine.adapter_info().name,
        "backend": format!("{:?}", h.engine.adapter_info().backend),
        "device_type": format!("{:?}", h.engine.adapter_info().device_type),
        "timing_scope": "execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU",
        "raw_fingerprint": fnv1a64(bytes),
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
        "instance_count": graph["operations"].as_array().unwrap().iter().map(|op| op["instance_count"].as_u64().unwrap_or(0)).sum::<u64>(),
        "pass_count": graph["passes"].as_array().unwrap().len()
    });
    fs::write(
        output_dir.join(format!("{output}_desktop.json")),
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .unwrap();
}

pub fn run_tc62() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc62_storage_texture.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let source = h.load_texture_exact("sprites_heroes.jpeg");
        let src_texture = h.registry.owned_texture(&source.handle).unwrap().clone();
        let src_view = src_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (out_handle, out_texture) = h.create_storage_texture(
            800,
            600,
            wgpu::TextureFormat::Rgba8Unorm,
            "TC62 Storage Texture Output",
        );
        let out_view = out_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let layout = h
            .engine
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tc62_storage_texture_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });
        let bind_group = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc62_storage_texture_bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&out_view),
                    },
                ],
            });
        let bind_group_handle = h.insert_bind_group(bind_group, 1);
        let pipeline = h.register_compute_pipeline("compute_storage_texture.wgsl", &[&layout]);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: out_handle,
            width: 800,
            height: 600,
        });
        graph.add_compute_batch(
            &mut pool,
            vec![ComputeCommand::new(pipeline, [50, 38, 1]).with_bind_group(
                0,
                bind_group_handle,
                Vec::new(),
            )],
        );
        let (cold_ms, cold_bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &out_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let (warm_ms, bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &out_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        assert!(
            bytes.iter().any(|byte| *byte != 0),
            "TC62 output must not be empty"
        );
        write_result(
            &h,
            "tc62_storage_texture",
            manifest_text,
            &manifest,
            &out_texture,
            wgpu::TextureFormat::Rgba8Unorm,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
        );
    });
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Particle {
    pos: [f32; 2],
    vel: [f32; 2],
    color: [f32; 4],
    life: f32,
    max_life: f32,
    size: f32,
    pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SimParams {
    delta_time: f32,
    attractor_count: u32,
    time: f32,
    damping: f32,
}

pub fn run_tc63() {
    pollster::block_on(async {
        const COUNT: usize = 100_000;
        let manifest_text = include_str!("../shared_assets/manifests/tc63_particles_100k.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let initial: Vec<Particle> = (0..COUNT)
            .map(|i| {
                let t = i as f32 / COUNT as f32;
                let angle = t * 6.2831853 * 4.0 + (i % 3) as f32 * 2.094;
                let radius = 0.15 + (i as f32 % 1000.0) / 1000.0 * 0.65;
                Particle {
                    pos: [angle.cos() * radius, angle.sin() * radius],
                    vel: [-angle.sin() * 0.6, angle.cos() * 0.6],
                    color: [0.2, 0.6, 1.0, 1.0],
                    life: 5.0,
                    max_life: 5.0,
                    size: 2.0,
                    pad: 0.0,
                }
            })
            .collect();
        let (_particle_handle, particle_buffer) =
            h.create_storage_buffer(&initial, "TC63 Particles", wgpu::BufferUsages::empty());
        let params = SimParams {
            delta_time: 0.016,
            attractor_count: 1,
            time: 2.5,
            damping: 0.992,
        };
        let params_buffer = h.engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TC63 Params"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        h.engine
            .queue()
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc63_compute_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let compute_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc63_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particle_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let compute_pipeline =
            h.register_compute_pipeline("compute_particles_100k.wgsl", &[&compute_layout]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc63_render_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });
        let render_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc63_render_bg"),
                layout: &render_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                }],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_particles_instanced.wgsl")
                .unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc63_render_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc63_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc63_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent::OVER,
                            }),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(1)]);
        let (target_handle, target_texture) = h.create_target("TC63 Particle Target");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.015, 0.018, 0.035, 1.0]);
        let mut computes = Vec::with_capacity(30);
        for _ in 0..30 {
            computes.push(
                ComputeCommand::new(compute_pipeline, [1563, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            );
        }
        graph.add_compute_batch(&mut pool, computes);
        graph.add_batch(
            &mut pool,
            vec![
                DrawCommand::new(
                    render_pipeline_handle,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..COUNT as u32,
                    },
                )
                .with_bind_group(0, render_bg_handle, Vec::new()),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        h.engine
            .queue()
            .write_buffer(&particle_buffer, 0, bytemuck::cast_slice(&initial));
        let (warm_ms, bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let final_particles: Vec<Particle> = h.readback_storage_buffer(&particle_buffer, COUNT);
        assert_eq!(
            final_particles
                .iter()
                .filter(|p| !p.pos[0].is_nan()
                    && !p.pos[1].is_nan()
                    && (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt() < 50.0)
                .count(),
            COUNT
        );
        write_result(
            &h,
            "tc63_particles_100k",
            manifest_text,
            &manifest,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
        );
    });
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct AudioParams {
    sample_count: u32,
    smoothing: f32,
    gain: f32,
    pad: f32,
}

pub fn run_tc64() {
    pollster::block_on(async {
        const SAMPLE_COUNT: usize = 4096;
        const BINS: usize = 64;
        let manifest_text = include_str!("../shared_assets/manifests/tc64_audio_fft.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let samples: Vec<f32> = (0..SAMPLE_COUNT)
            .map(|i| {
                let t = i as f32 / 44100.0;
                let noise = ((i * 73 % 100) as f32 / 100.0 - 0.5) * 0.05;
                (2.0 * std::f32::consts::PI * 120.0 * t).sin() * 0.55
                    + (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.45
                    + (2.0 * std::f32::consts::PI * 1800.0 * t).sin() * 0.35
                    + noise
            })
            .collect();
        let (_audio_handle, audio_buffer) =
            h.create_storage_buffer(&samples, "TC64 Audio Samples", wgpu::BufferUsages::empty());
        let zero = vec![0.0f32; BINS];
        let (_spectrum_handle, spectrum_buffer) =
            h.create_storage_buffer(&zero, "TC64 Spectrum", wgpu::BufferUsages::empty());
        let params = AudioParams {
            sample_count: SAMPLE_COUNT as u32,
            smoothing: 0.8,
            gain: 1.35,
            pad: 0.0,
        };
        let params_buffer = h.engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TC64 Params"),
            size: std::mem::size_of::<AudioParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        h.engine
            .queue()
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc64_compute_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let compute_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc64_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: audio_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: spectrum_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let compute_pipeline =
            h.register_compute_pipeline("compute_audio_fft.wgsl", &[&compute_layout]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc64_render_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });
        let render_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc64_render_bg"),
                layout: &render_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: audio_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: spectrum_buffer.as_entire_binding(),
                    },
                ],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_audio_spectrum.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc64_render_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc64_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc64_render_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(1)]);
        let (target_handle, target_texture) = h.create_target("TC64 Visualizer Target");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph.add_compute_batch(
            &mut pool,
            vec![
                ComputeCommand::new(compute_pipeline, [1, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            ],
        );
        graph.add_batch(
            &mut pool,
            vec![
                DrawCommand::new(
                    render_pipeline_handle,
                    DrawAction::Procedural {
                        vertex_count: 3,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, render_bg_handle, Vec::new()),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let (warm_ms, bytes) = execute_graphs(
            &mut h,
            &mut pool,
            &[&graph],
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let spectrum: Vec<f32> = h.readback_storage_buffer(&spectrum_buffer, BINS);
        assert!(
            spectrum.iter().copied().fold(0.0, f32::max) > 0.5,
            "TC64 FFT must produce a significant peak"
        );
        write_result(
            &h,
            "tc64_audio_fft",
            manifest_text,
            &manifest,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
        );
    });
}
