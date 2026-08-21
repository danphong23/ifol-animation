#![allow(dead_code)]

use super::advanced_effects::fnv1a64;
use super::harness::DesktopTestHarness;
use bytemuck::{Pod, Zeroable, bytes_of, cast_slice};
use ifol_gpu::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use serde_json::Value;
use std::fs;
use std::time::Instant;
use wgpu::util::DeviceExt;

fn parse_manifest(text: &'static str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn execute_graph(
    h: &mut DesktopTestHarness,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    target: &wgpu::Texture,
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
        .read_texture_to_raw_with_format_checked(target, wgpu::TextureFormat::Rgba8UnormSrgb)
        .unwrap()
        .bytes;
    (elapsed, bytes)
}

#[expect(clippy::too_many_arguments, reason = "test report helper keeps evidence fields explicit")]
fn write_result(
    h: &DesktopTestHarness,
    output: &str,
    manifest_text: &str,
    manifest: &Value,
    target: &wgpu::Texture,
    cold_ms: f64,
    cold_bytes: &[u8],
    warm_ms: f64,
    bytes: &[u8],
    numeric_validation: Value,
) {
    assert_eq!(
        cold_bytes, bytes,
        "{output} output changed between reset cold and warm runs"
    );
    let output_dir = std::path::Path::new("tests/outputs/desktop");
    fs::create_dir_all(output_dir).unwrap();
    h.save_texture_to_file_checked(
        target,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        output_dir.join(format!("{output}.png")),
    )
    .unwrap();
    fs::write(output_dir.join(format!("{output}_desktop.bin")), bytes).unwrap();
    let graph = &manifest["graph"];
    let metadata = serde_json::json!({
        "test_case": manifest["test_case"],
        "width": graph["target"]["width"],
        "height": graph["target"]["height"],
        "format": "Rgba8UnormSrgb",
        "adapter_name": h.engine.adapter_info().name,
        "backend": format!("{:?}", h.engine.adapter_info().backend),
        "device_type": format!("{:?}", h.engine.adapter_info().device_type),
        "timing_scope": "execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        "isolation_scope": "DesktopTestHarness mới cho từng TC; state mutable được reset trước warm; không xóa cache nội bộ của driver/GPU",
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
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct VerletNode {
    pos: [f32; 2],
    prev_pos: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct TimeUniform {
    time: f32,
    pad: [f32; 3],
}

fn initial_verlet_nodes() -> Vec<VerletNode> {
    let mut nodes = vec![VerletNode::zeroed(); 4096];
    for chain in 0..256 {
        let anchor_x = (chain % 16) as f32 * 50.0 + 25.0;
        let anchor_y = (chain / 16) as f32 * 10.0 + 50.0;
        for node in 0..16 {
            let y = anchor_y + node as f32 * 20.0;
            nodes[chain * 16 + node] = VerletNode {
                pos: [anchor_x, y],
                prev_pos: [anchor_x, y],
            };
        }
    }
    nodes
}

pub fn run_tc68() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc68_verlet.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let initial_nodes = initial_verlet_nodes();
        let (_, nodes_buffer) = h.create_storage_buffer(
            &initial_nodes,
            "TC68 Verlet Nodes",
            wgpu::BufferUsages::empty(),
        );
        let uniform = TimeUniform {
            time: 5.0,
            pad: [0.0; 3],
        };
        let uniform_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC68 Time"),
                    contents: bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc68_compute_layout"),
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
                label: Some("tc68_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: nodes_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_verlet.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc68_compute_verlet"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let compute_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc68_compute_pipeline_layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });
        let integrate =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc68_integrate"),
                    layout: Some(&compute_pipeline_layout),
                    module: &shader,
                    entry_point: Some("integrate_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let constrain =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc68_constrain"),
                    layout: Some(&compute_pipeline_layout),
                    module: &shader,
                    entry_point: Some("constrain_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let integrate_handle = h.insert_compute_pipeline(integrate, vec![Some(1)]);
        let constrain_handle = h.insert_compute_pipeline(constrain, vec![Some(1)]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc68_render_layout"),
                    entries: [wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }]
                    .as_ref(),
                });
        let render_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc68_render_bg"),
                layout: &render_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: nodes_buffer.as_entire_binding(),
                }],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 1);
        let render_shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_chains.wgsl").unwrap();
        let render_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc68_render_chains"),
                source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
            });
        let render_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc68_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc68_render_pipeline"),
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
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(1)]);
        let (target_handle, target_texture) = h.create_target("TC68 Verlet Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);
        let mut commands = Vec::with_capacity(200);
        for _ in 0..100 {
            commands.push(
                ComputeCommand::new(integrate_handle, [64, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            );
            commands.push(
                ComputeCommand::new(constrain_handle, [4, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            );
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
        let (cold_ms, cold_bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        h.engine
            .queue()
            .write_buffer(&nodes_buffer, 0, cast_slice(&initial_nodes));
        let (warm_ms, bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        let final_nodes = h.readback_storage_buffer::<VerletNode>(&nodes_buffer, 4096);
        assert!(
            final_nodes
                .iter()
                .all(|node| node.pos.iter().all(|v| v.is_finite()))
        );
        write_result(
            &h,
            "tc68_verlet",
            manifest_text,
            &manifest,
            &target_texture,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            serde_json::json!({ "node_count": 4096, "finite_node_count": final_nodes.iter().filter(|node| node.pos.iter().all(|v| v.is_finite())).count(), "seed_reset_before_warm": true }),
        );
    });
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct DeformVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

fn initial_deformation_grid() -> (Vec<DeformVertex>, Vec<u16>) {
    let grid = 64usize;
    let mut vertices = Vec::with_capacity(4225);
    for y in 0..=grid {
        for x in 0..=grid {
            let tx = x as f32 / grid as f32;
            let ty = y as f32 / grid as f32;
            vertices.push(DeformVertex {
                pos: [tx * 2.0 - 1.0, ty * 2.0 - 1.0],
                uv: [tx, ty],
                color: [0.5, 0.5, 0.5, 1.0],
            });
        }
    }
    let mut indices = Vec::with_capacity(24576);
    for y in 0..grid {
        for x in 0..grid {
            let tl = y * (grid + 1) + x;
            let tr = tl + 1;
            let bl = (y + 1) * (grid + 1) + x;
            let br = bl + 1;
            indices.extend_from_slice(&[
                tl as u16, bl as u16, tr as u16, tr as u16, bl as u16, br as u16,
            ]);
        }
    }
    (vertices, indices)
}

pub fn run_tc69() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc69_deformation.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let (initial_vertices, indices) = initial_deformation_grid();
        let (_, src_buffer) = h.create_storage_buffer(
            &initial_vertices,
            "TC69 Source Vertices",
            wgpu::BufferUsages::empty(),
        );
        let (_, dest_buffer) = h.create_storage_buffer(
            &initial_vertices,
            "TC69 Dest Vertices",
            wgpu::BufferUsages::VERTEX,
        );
        let index_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC69 Indices"),
                    contents: cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
        let uniform = TimeUniform {
            time: 5.0,
            pad: [0.0; 3],
        };
        let uniform_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC69 Time"),
                    contents: bytes_of(&uniform),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc69_compute_layout"),
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
                label: Some("tc69_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: src_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: dest_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let compute_shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_deformation.wgsl").unwrap();
        let compute_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc69_compute_deformation"),
                source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
            });
        let compute_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc69_compute_pipeline_layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });
        let compute_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc69_compute_pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_shader,
                    entry_point: Some("cs_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let compute_pipeline_handle = h.insert_compute_pipeline(compute_pipeline, vec![Some(1)]);
        let render_shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_deformation.wgsl").unwrap();
        let render_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc69_render_deformation"),
                source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
            });
        let render_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc69_render_pipeline_layout"),
                    bind_group_layouts: &[],
                    immediate_size: 0,
                });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<DeformVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc69_render_pipeline"),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[Some(vertex_layout)],
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
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        cull_mode: None,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, Vec::new());
        let mesh_handle = h.insert_mesh(
            dest_buffer.clone(),
            Some((index_buffer, wgpu::IndexFormat::Uint16)),
            4225,
        );
        let (target_handle, target_texture) = h.create_target("TC69 Deformation Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);
        graph.add_compute_batch(
            &mut pool,
            vec![
                ComputeCommand::new(compute_pipeline_handle, [67, 1, 1]).with_bind_group(
                    0,
                    compute_bg_handle,
                    Vec::new(),
                ),
            ],
        );
        graph.add_batch(
            &mut pool,
            vec![DrawCommand::new(
                render_pipeline_handle,
                DrawAction::Indexed {
                    mesh: mesh_handle,
                    index_range: 0..24576,
                    instance_range: 0..1,
                },
            )],
        );
        let (cold_ms, cold_bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        h.engine
            .queue()
            .write_buffer(&dest_buffer, 0, cast_slice(&initial_vertices));
        let (warm_ms, bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        assert!(bytes.iter().any(|byte| *byte != 0));
        write_result(
            &h,
            "tc69_deformation",
            manifest_text,
            &manifest,
            &target_texture,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            serde_json::json!({ "vertex_count": 4225, "index_count": 24576, "zero_copy_dest_vertex_buffer": true, "seed_reset_before_warm": true }),
        );
    });
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CullParticle {
    pos: [f32; 2],
    radius: f32,
    pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct IndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

fn initial_cull_particles() -> Vec<CullParticle> {
    (0..100_000)
        .map(|i| CullParticle {
            pos: [
                ((i * 13) % 1000) as f32 / 1000.0 * 2.0 - 1.0,
                ((i * 17) % 1000) as f32 / 1000.0 * 2.0 - 1.0,
            ],
            radius: 0.005,
            pad: 0.0,
        })
        .collect()
}

pub fn run_tc70() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc70_culling.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let particles = initial_cull_particles();
        let (_, src_buffer) = h.create_storage_buffer(
            &particles,
            "TC70 Source Particles",
            wgpu::BufferUsages::empty(),
        );
        let (_, dst_buffer) = h.create_storage_buffer(
            &particles,
            "TC70 Compact Particles",
            wgpu::BufferUsages::empty(),
        );
        let indirect_initial = IndirectArgs {
            vertex_count: 6,
            instance_count: 0,
            first_vertex: 0,
            first_instance: 0,
        };
        let (indirect_handle, indirect_buffer) = h.create_storage_buffer(
            &[indirect_initial],
            "TC70 Indirect Args",
            wgpu::BufferUsages::INDIRECT,
        );
        let uniform = [0.0f32, 0.0, 0.5, 0.0];
        let (_, uniform_buffer) =
            h.create_storage_buffer(&uniform, "TC70 Cull Uniform", wgpu::BufferUsages::UNIFORM);
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc70_compute_layout"),
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
        let indirect_ref = h.registry.buffer(&indirect_handle).unwrap();
        let compute_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc70_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: src_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: dst_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: indirect_ref.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let compute_shader_code =
            fs::read_to_string("tests/shared_assets/shaders/compute_cull.wgsl").unwrap();
        let compute_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc70_compute_cull"),
                source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
            });
        let compute_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc70_compute_pipeline_layout"),
                    bind_group_layouts: &[Some(&compute_layout)],
                    immediate_size: 0,
                });
        let compute_pipeline =
            h.engine
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("tc70_compute_pipeline"),
                    layout: Some(&compute_pipeline_layout),
                    module: &compute_shader,
                    entry_point: Some("cs_main"),
                    compilation_options: Default::default(),
                    cache: None,
                });
        let compute_pipeline_handle = h.insert_compute_pipeline(compute_pipeline, vec![Some(1)]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc70_render_layout"),
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
                label: Some("tc70_render_bg"),
                layout: &render_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: dst_buffer.as_entire_binding(),
                }],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 2);
        let render_shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_culled.wgsl").unwrap();
        let render_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc70_render_culled"),
                source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
            });
        let render_pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc70_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc70_render_pipeline"),
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
        let (target_handle, target_texture) = h.create_target("TC70 Culling Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);
        graph.add_compute_batch(
            &mut pool,
            vec![
                ComputeCommand::new(compute_pipeline_handle, [1563, 1, 1]).with_bind_group(
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
                    DrawAction::Indirect {
                        buffer: indirect_handle,
                        offset: 0,
                    },
                )
                .with_bind_group(0, render_bg_handle, Vec::new()),
            ],
        );
        let (cold_ms, cold_bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        h.engine
            .queue()
            .write_buffer(&indirect_buffer, 0, bytes_of(&indirect_initial));
        let (warm_ms, bytes) = execute_graph(&mut h, &mut pool, &graph, &target_texture);
        let args = h.readback_storage_buffer::<IndirectArgs>(&indirect_buffer, 1)[0];
        let expected = particles
            .iter()
            .filter(|p| {
                let dx = p.pos[0] * (800.0 / 600.0);
                (dx * dx + p.pos[1] * p.pos[1]).sqrt() <= 0.5
            })
            .count() as u32;
        assert_eq!(args.instance_count, expected);
        write_result(
            &h,
            "tc70_culling",
            manifest_text,
            &manifest,
            &target_texture,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            serde_json::json!({ "input_count": 100000, "expected_instance_count": expected, "actual_instance_count": args.instance_count, "indirect_draw": true, "counter_reset_before_warm": true }),
        );
    });
}
