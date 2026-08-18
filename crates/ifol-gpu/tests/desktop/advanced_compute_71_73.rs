#![allow(dead_code)]

use super::advanced_effects::fnv1a64;
use super::harness::DesktopTestHarness;
use bytemuck::{Pod, Zeroable, bytes_of, cast_slice};
use ifol_gpu::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use ifol_gpu::resources::{BindGroupHandle, BindGroupResourceDescriptor};
use serde_json::Value;
use std::fs;
use std::time::Instant;
use wgpu::util::DeviceExt;

fn manifest(text: &'static str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn execute_graph(
    h: &mut DesktopTestHarness,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    target: &wgpu::Texture,
    format: wgpu::TextureFormat,
) -> (f64, Vec<u8>) {
    let started = Instant::now();
    let submission = h
        .executor
        .execute_checked(&h.engine, &h.registry, pool, graph)
        .unwrap();
    let _ = h.engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let elapsed = started.elapsed().as_secs_f64() * 1000.0;
    let bytes = h
        .engine
        .read_texture_to_raw_with_format_checked(target, format)
        .unwrap()
        .bytes;
    (elapsed, bytes)
}

#[derive(Debug, Clone, Copy)]
struct WarmComparison {
    equal: bool,
    different_bytes: usize,
    different_pixels: usize,
    max_channel_delta: u8,
}

fn compare_warm_output(cold: &[u8], warm: &[u8]) -> WarmComparison {
    let different_bytes = cold
        .iter()
        .zip(warm.iter())
        .filter(|(left, right)| left != right)
        .count()
        + cold.len().abs_diff(warm.len());
    let different_pixels = cold
        .chunks_exact(4)
        .zip(warm.chunks_exact(4))
        .filter(|(left, right)| left != right)
        .count()
        + cold.len().abs_diff(warm.len()) / 4;
    let max_channel_delta = cold
        .iter()
        .zip(warm.iter())
        .map(|(left, right)| left.abs_diff(*right))
        .max()
        .unwrap_or(0);
    WarmComparison {
        equal: cold == warm,
        different_bytes,
        different_pixels,
        max_channel_delta,
    }
}

fn write_result(
    h: &DesktopTestHarness,
    output: &str,
    manifest_text: &str,
    spec: &Value,
    target: &wgpu::Texture,
    format: wgpu::TextureFormat,
    cold_ms: f64,
    cold_bytes: &[u8],
    warm_ms: f64,
    warm_bytes: &[u8],
    numeric_validation: Value,
) {
    let warm_comparison = compare_warm_output(cold_bytes, warm_bytes);
    let warm_pixel_tolerance = spec["evaluation"]
        .get("warm_pixel_diff_tolerance")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    assert!(
        warm_comparison.different_pixels <= warm_pixel_tolerance,
        "{output} output changed beyond declared warm tolerance: {} pixels, tolerance {}",
        warm_comparison.different_pixels,
        warm_pixel_tolerance
    );
    let output_dir = std::path::Path::new("tests/outputs/desktop");
    fs::create_dir_all(output_dir).unwrap();
    h.save_texture_to_file_checked(target, format, output_dir.join(format!("{output}.png")))
        .unwrap();
    fs::write(output_dir.join(format!("{output}_desktop.bin")), warm_bytes).unwrap();
    let graph = &spec["graph"];
    let metadata = serde_json::json!({
        "test_case": spec["test_case"],
        "width": graph["target"]["width"],
        "height": graph["target"]["height"],
        "format": format!("{format:?}"),
        "adapter_name": h.engine.adapter_info().name,
        "backend": format!("{:?}", h.engine.adapter_info().backend),
        "device_type": format!("{:?}", h.engine.adapter_info().device_type),
        "timing_scope": "execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        "isolation_scope": "DesktopTestHarness mới cho từng TC; state mutable được reset hoặc ghi đè trước warm; không xóa cache nội bộ của driver/GPU",
        "raw_fingerprint": fnv1a64(warm_bytes),
        "manifest": format!("tests/shared_assets/manifests/{output}.json"),
        "manifest_fingerprint": fnv1a64(manifest_text.as_bytes()),
        "cold_render_time_ms": cold_ms,
        "warm_render_time_ms": warm_ms,
        "warm_iteration_count": 1,
        "speedup_percentage": (1.0 - warm_ms / cold_ms) * 100.0,
        "cache_output_equal": warm_comparison.equal,
        "warm_output_within_tolerance": true,
        "warm_diff_bytes": warm_comparison.different_bytes,
        "warm_diff_pixels": warm_comparison.different_pixels,
        "warm_max_channel_delta": warm_comparison.max_channel_delta,
        "warm_pixel_diff_tolerance": warm_pixel_tolerance,
        "validation_passed": true,
        "validation_error": Value::Null,
        "numeric_validation": numeric_validation,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SortParticle {
    pos: [f32; 2],
    depth: f32,
    _pad: f32,
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SortParams {
    j: u32,
    k: u32,
}

fn initial_sort_particles() -> Vec<SortParticle> {
    (0..65_536)
        .map(|i| {
            let r1 = (i * 13 % 1000) as f32 / 1000.0;
            let r2 = (i * 17 % 1000) as f32 / 1000.0;
            let depth = (i * 23 % 1000) as f32 / 1000.0;
            SortParticle {
                pos: [r1 * 2.0 - 1.0, r2 * 2.0 - 1.0],
                depth,
                _pad: 0.0,
                color: [depth, 0.0, 1.0 - depth, 1.0],
            }
        })
        .collect()
}

pub fn run_tc71() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc71_bitonic_sort.json");
        let spec = manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let initial = initial_sort_particles();
        let (_, source_buffer) = h.create_storage_buffer(
            &initial,
            "TC71 Bitonic Source",
            wgpu::BufferUsages::empty(),
        );
        let (_, destination_buffer) = h.create_storage_buffer(
            &initial,
            "TC71 Bitonic Destination",
            wgpu::BufferUsages::empty(),
        );
        let alignment = 256usize;
        let mut params = Vec::new();
        let mut k = 2usize;
        while k <= 65_536 {
            let mut j = k >> 1;
            while j > 0 {
                params.push(SortParams {
                    j: j as u32,
                    k: k as u32,
                });
                j >>= 1;
            }
            k <<= 1;
        }
        assert_eq!(params.len(), 136);
        let mut uniform_data = vec![0u8; params.len() * alignment];
        for (index, param) in params.iter().enumerate() {
            uniform_data[index * alignment..index * alignment + std::mem::size_of::<SortParams>()]
                .copy_from_slice(bytes_of(param));
        }
        let uniform_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC71 Sort Params"),
                    contents: &uniform_data,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc71_compute_layout"),
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
                                has_dynamic_offset: true,
                                min_binding_size: wgpu::BufferSize::new(8),
                            },
                            count: None,
                        },
                    ],
                });
        let make_compute_bg = |label: &str, source: &wgpu::Buffer, destination: &wgpu::Buffer| {
            h.engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: source.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: destination.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &uniform_buffer,
                            offset: 0,
                            size: wgpu::BufferSize::new(8),
                        }),
                    },
                ],
            })
        };
        let compute_bg_source_to_destination = make_compute_bg(
            "tc71_compute_source_to_destination",
            &source_buffer,
            &destination_buffer,
        );
        let compute_bg_destination_to_source = make_compute_bg(
            "tc71_compute_destination_to_source",
            &destination_buffer,
            &source_buffer,
        );
        let compute_bg_source_to_destination_handle = BindGroupHandle(998);
        let compute_bg_destination_to_source_handle = BindGroupHandle(999);
        h.registry
            .insert_bind_group_with_descriptor(
                compute_bg_source_to_destination_handle,
                compute_bg_source_to_destination,
                BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment as u32,
                    layout_signature: 1,
                },
            )
            .unwrap();
        h.registry
            .insert_bind_group_with_descriptor(
                compute_bg_destination_to_source_handle,
                compute_bg_destination_to_source,
                BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment as u32,
                    layout_signature: 1,
                },
            )
            .unwrap();
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_bitonic_sort.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc71_sort_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc71_compute_pipeline_layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });
        let compute_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc71_sort_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let compute_pipeline_handle = h.insert_compute_pipeline(compute_pipeline, vec![Some(1)]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc71_render_layout"),
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
                label: Some("tc71_render_bg"),
                layout: &render_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                }],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 2);
        let render_code =
            fs::read_to_string("tests/shared_assets/shaders/render_bitonic_sort.wgsl").unwrap();
        let render_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc71_render_shader"),
                source: wgpu::ShaderSource::Wgsl(render_code.into()),
            });
        let render_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc71_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc71_render_pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(2)]);
        let (target_handle, target_texture) = h.create_target("TC71 Bitonic Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);
        let compute_commands = params
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let bind_group_handle = if index % 2 == 0 {
                    compute_bg_source_to_destination_handle
                } else {
                    compute_bg_destination_to_source_handle
                };
                ComputeCommand::new(compute_pipeline_handle, [256, 1, 1]).with_bind_group(
                    0,
                    bind_group_handle,
                    vec![(index * alignment) as u32],
                )
            })
            .collect();
        graph.add_compute_batch(&mut pool, compute_commands);
        graph.add_batch(
            &mut pool,
            vec![
                DrawCommand::new(
                    render_pipeline_handle,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..65_536,
                    },
                )
                .with_bind_group(0, render_bg_handle, Vec::new()),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        h.engine
            .queue()
            .write_buffer(&source_buffer, 0, cast_slice(&initial));
        h.engine
            .queue()
            .write_buffer(&destination_buffer, 0, cast_slice(&initial));
        let (warm_ms, warm_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let sorted = h.readback_storage_buffer::<SortParticle>(&source_buffer, 65_536);
        assert!(sorted.iter().all(|particle| particle.depth.is_finite()));
        assert!(
            sorted
                .windows(2)
                .all(|pair| pair[0].depth <= pair[1].depth + 1e-6)
        );
        write_result(
            &h,
            "tc71_bitonic_sort",
            manifest_text,
            &spec,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &warm_bytes,
            serde_json::json!({ "particle_count": 65536, "stage_count": 136, "sorted_non_decreasing": true, "first_depth": sorted[0].depth, "last_depth": sorted[65535].depth, "seed_reset_before_warm": true }),
        );
    });
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SpatialParticle {
    pos: [f32; 2],
    vel: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct SpatialParams {
    num_particles: u32,
    grid_size: u32,
    cell_size: f32,
    radius: f32,
    dt: f32,
    _pad: [f32; 3],
}

fn initial_spatial_particles() -> Vec<SpatialParticle> {
    (0..4096)
        .map(|i| SpatialParticle {
            pos: [(i % 64) as f32 * 12.0 + 10.0, (i / 64) as f32 * 12.0 + 10.0],
            vel: [((i % 3) as f32 - 1.0) * 10.0, ((i % 5) as f32 - 2.0) * 10.0],
            color: [1.0; 4],
        })
        .collect()
}

pub fn run_tc72() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc72_spatial_hash.json");
        let spec = manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 800).await;
        let initial = initial_spatial_particles();
        let (_, source_buffer) =
            h.create_storage_buffer(&initial, "TC72 Particles A", wgpu::BufferUsages::VERTEX);
        let (_, destination_buffer) =
            h.create_storage_buffer(&initial, "TC72 Particles B", wgpu::BufferUsages::VERTEX);
        let grid_bytes = vec![0u8; 32 * 32 * 144];
        let (_, grid_buffer) = h.create_storage_buffer(
            &grid_bytes,
            "TC72 Spatial Grid",
            wgpu::BufferUsages::empty(),
        );
        let params = SpatialParams {
            num_particles: 4096,
            grid_size: 32,
            cell_size: 25.0,
            radius: 4.0,
            dt: 0.16,
            _pad: [0.0; 3],
        };
        let uniform_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC72 Params"),
                    contents: bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc72_compute_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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
        let compute_bg_a_to_b = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc72_compute_bg_a_to_b"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: source_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: destination_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: grid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_b_to_a = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc72_compute_bg_b_to_a"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: destination_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: source_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: grid_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_a_to_b_handle = h.insert_bind_group(compute_bg_a_to_b, 1);
        let compute_bg_b_to_a_handle = h.insert_bind_group(compute_bg_b_to_a, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_spatial_hash.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc72_compute_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc72_compute_pipeline_layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });
        let reset_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc72_reset"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_reset_grid"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let hash_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc72_hash"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_hash_particles"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let simulate_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc72_simulate"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_simulate"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let reset_handle = h.insert_compute_pipeline(reset_pipeline, vec![Some(1)]);
        let hash_handle = h.insert_compute_pipeline(hash_pipeline, vec![Some(1)]);
        let simulate_handle = h.insert_compute_pipeline(simulate_pipeline, vec![Some(1)]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc72_render_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                label: Some("tc72_render_bg"),
                layout: &render_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                }],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 2);
        let render_code =
            fs::read_to_string("tests/shared_assets/shaders/render_spatial_hash.wgsl").unwrap();
        let render_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc72_render_shader"),
                source: wgpu::ShaderSource::Wgsl(render_code.into()),
            });
        let render_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc72_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc72_render_pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader,
                        entry_point: Some("fs_main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: wgpu::TextureFormat::Rgba8UnormSrgb,
                            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(2)]);
        let (target_handle, target_texture) = h.create_target("TC72 Spatial Hash Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 800,
        })
        .with_clear_color([0.02, 0.02, 0.04, 1.0]);
        let mut commands = Vec::with_capacity(30);
        for iteration in 0..10 {
            let compute_bg_handle = if iteration % 2 == 0 {
                compute_bg_a_to_b_handle
            } else {
                compute_bg_b_to_a_handle
            };
            commands.extend([
                ComputeCommand::new(reset_handle, [16, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
                ComputeCommand::new(hash_handle, [64, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
                ComputeCommand::new(simulate_handle, [64, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            ]);
        }
        graph.add_compute_batch(&mut pool, commands);
        graph.add_batch(
            &mut pool,
            vec![
                DrawCommand::new(
                    render_pipeline_handle,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..4096,
                    },
                )
                .with_bind_group(0, render_bg_handle, Vec::new()),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        h.engine
            .queue()
            .write_buffer(&source_buffer, 0, cast_slice(&initial));
        h.engine
            .queue()
            .write_buffer(&destination_buffer, 0, cast_slice(&initial));
        h.engine.queue().write_buffer(&grid_buffer, 0, &grid_bytes);
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        });
        let (warm_ms, warm_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let final_particles = h.readback_storage_buffer::<SpatialParticle>(&source_buffer, 4096);
        assert!(final_particles.iter().all(|p| p.pos.iter().all(|v| v.is_finite()) && p.vel.iter().all(|v| v.is_finite())));
        assert!(
            final_particles
                .iter()
                .all(|p| p.pos.iter().all(|v| *v >= 4.0 && *v <= 796.0))
        );
        write_result(
            &h,
            "tc72_spatial_hash",
            manifest_text,
            &spec,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &warm_bytes,
            serde_json::json!({ "particle_count": 4096, "grid_size": 32, "iteration_count": 10, "particle_buffers": 2, "state_update": "source_to_destination_ping_pong", "finite_particle_count": final_particles.len(), "bounded_particle_count": final_particles.iter().filter(|p| p.pos.iter().all(|v| *v >= 4.0 && *v <= 796.0)).count(), "seed_reset_before_warm": true }),
        );
    });
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct MorphParams {
    radius: i32,
    mode: i32,
    _pad: [i32; 2],
}

pub fn run_tc73() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc73_morphology.json");
        let spec = manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 800).await;
        let (_, mask_texture) =
            h.create_storage_texture(800, 800, wgpu::TextureFormat::Rgba8Unorm, "TC73 Mask");
        let (target_handle, target_texture) = h.create_storage_texture(
            800,
            800,
            wgpu::TextureFormat::Rgba8Unorm,
            "TC73 Morphology Output",
        );
        let params = MorphParams {
            radius: 10,
            mode: 0,
            _pad: [0; 2],
        };
        let uniform_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC73 Params"),
                    contents: bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let mask_view = mask_texture.create_view(&Default::default());
        let target_view = target_texture.create_view(&Default::default());
        let gen_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc73_gen_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    }],
                });
        let morph_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc73_morph_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
        let gen_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc73_gen_bg"),
                layout: &gen_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&mask_view),
                }],
            });
        let morph_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc73_morph_bg"),
                layout: &morph_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&mask_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&target_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let gen_bg_handle = h.insert_bind_group(gen_bg, 1);
        let morph_bg_handle = h.insert_bind_group(morph_bg, 2);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_morphology.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc73_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let gen_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc73_gen_pipeline_layout"),
                    bind_group_layouts: &[Some(&gen_layout)],
                    immediate_size: 0,
                });
        let morph_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc73_morph_pipeline_layout"),
                    bind_group_layouts: &[Some(&morph_layout)],
                    immediate_size: 0,
                });
        let gen_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc73_gen_pipeline"),
                    layout: Some(&gen_pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_gen_mask"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let morph_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc73_morph_pipeline"),
                    layout: Some(&morph_pipeline_layout),
                    module: &shader,
                    entry_point: Some("cs_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let gen_handle = h.insert_compute_pipeline(gen_pipeline, vec![Some(1)]);
        let morph_handle = h.insert_compute_pipeline(morph_pipeline, vec![Some(2)]);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 800,
        });
        graph.add_compute_batch(
            &mut pool,
            vec![
                ComputeCommand::new(gen_handle, [50, 50, 1]).with_bind_group(
                    0,
                    gen_bg_handle,
                    Vec::new(),
                ),
                ComputeCommand::new(morph_handle, [50, 50, 1]).with_bind_group(
                    0,
                    morph_bg_handle,
                    Vec::new(),
                ),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let (warm_ms, warm_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let nonzero = warm_bytes
            .chunks_exact(4)
            .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
            .count();
        assert!(nonzero > 0);
        write_result(
            &h,
            "tc73_morphology",
            manifest_text,
            &spec,
            &target_texture,
            wgpu::TextureFormat::Rgba8Unorm,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &warm_bytes,
            serde_json::json!({ "width": 800, "height": 800, "radius": 10, "mode": "dilation", "nonzero_pixel_count": nonzero, "mask_rewritten_before_warm": true }),
        );
    });
}
