#![allow(dead_code)]

use super::advanced_effects::fnv1a64;
use super::harness::DesktopTestHarness;
use bytemuck::cast_slice;
use ifol_gpu::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use serde_json::Value;
use std::fs;
use std::time::Instant;

fn parse_manifest(text: &'static str) -> Value {
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

#[expect(clippy::too_many_arguments, reason = "test report helper keeps evidence fields explicit")]
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
    numeric_validation: Value,
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

pub fn run_tc65() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc65_workgroup_blur.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let source = h.load_texture_exact("sprites_heroes.jpeg");
        let src_texture = h.registry.owned_texture(&source.handle).unwrap().clone();
        let src_view = src_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let (output_handle, output_texture) = h.create_storage_texture(
            800,
            600,
            wgpu::TextureFormat::Rgba8Unorm,
            "TC65 Workgroup Blur Output",
        );
        let output_view = output_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let layout = h
            .engine
            .device()
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("tc65_workgroup_blur_layout"),
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
                label: Some("tc65_workgroup_blur_bind_group"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&output_view),
                    },
                ],
            });
        let bind_group_handle = h.insert_bind_group(bind_group, 1);
        let pipeline = h.register_compute_pipeline("compute_workgroup_blur.wgsl", &[&layout]);
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: output_handle,
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
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &output_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let (warm_ms, bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &output_texture,
            wgpu::TextureFormat::Rgba8Unorm,
        );
        assert!(bytes.iter().any(|byte| *byte != 0));
        write_result(
            &h,
            "tc65_workgroup_blur",
            manifest_text,
            &manifest,
            &output_texture,
            wgpu::TextureFormat::Rgba8Unorm,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            Value::Null,
        );
    });
}

pub fn run_tc66() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc66_histogram.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let source = h.load_texture_exact("sprites_heroes.jpeg");
        let src_texture = h.registry.owned_texture(&source.handle).unwrap().clone();
        let src_view = src_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let zeros = vec![0u32; 256];
        let (_, histogram_buffer) =
            h.create_storage_buffer(&zeros, "TC66 Histogram", wgpu::BufferUsages::empty());
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc66_histogram_compute_layout"),
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
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
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
                label: Some("tc66_histogram_compute_bg"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: histogram_buffer.as_entire_binding(),
                    },
                ],
            });
        let compute_bg_handle = h.insert_bind_group(compute_bg, 1);
        let compute_pipeline =
            h.register_compute_pipeline("compute_histogram.wgsl", &[&compute_layout]);

        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc66_histogram_render_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
                label: Some("tc66_histogram_render_bg"),
                layout: &render_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&src_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&h.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: histogram_buffer.as_entire_binding(),
                    },
                ],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_histogram.wgsl").unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc66_render_histogram"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc66_render_histogram_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc66_render_histogram_pipeline"),
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
                    primitive: Default::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                });
        let render_pipeline_handle = h.insert_pipeline(render_pipeline, vec![Some(1)]);
        let (target_handle, target_texture) = h.create_target("TC66 Histogram Output");
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
                ComputeCommand::new(compute_pipeline, [50, 38, 1]).with_bind_group(
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
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        h.engine
            .queue()
            .write_buffer(&histogram_buffer, 0, cast_slice(&zeros));
        let (warm_ms, bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let histogram = h.readback_storage_buffer::<u32>(&histogram_buffer, 256);
        let total: u32 = histogram.iter().sum();
        assert_eq!(total, 800 * 600, "TC66 histogram must count every pixel");
        write_result(
            &h,
            "tc66_histogram",
            manifest_text,
            &manifest,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            serde_json::json!({ "element_count": 256, "pixel_count": 800 * 600, "histogram_sum": total }),
        );
    });
}

fn seed_texture(width: u32, height: u32) -> Vec<u8> {
    let mut seed = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            seed[index] = 255;
            seed[index + 3] = 255;
            if (x > 380 && x < 420 && y > 280 && y < 320)
                || (x > 200 && x < 220 && y > 400 && y < 420)
                || (x > 600 && x < 620 && y > 150 && y < 170)
            {
                seed[index + 1] = 255;
            }
        }
    }
    seed
}

fn upload_seed(queue: &wgpu::Queue, texture: &wgpu::Texture, seed: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        seed,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * 800),
            rows_per_image: Some(600),
        },
        wgpu::Extent3d {
            width: 800,
            height: 600,
            depth_or_array_layers: 1,
        },
    );
}

pub fn run_tc67() {
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc67_pingpong.json");
        let manifest = parse_manifest(manifest_text);
        let mut h = DesktopTestHarness::new(800, 600).await;
        let (texture_a_handle, texture_a) = h.create_storage_texture(
            800,
            600,
            wgpu::TextureFormat::Rgba8Unorm,
            "TC67 Reaction Texture A",
        );
        let (texture_b_handle, texture_b) = h.create_storage_texture(
            800,
            600,
            wgpu::TextureFormat::Rgba8Unorm,
            "TC67 Reaction Texture B",
        );
        let seed = seed_texture(800, 600);
        upload_seed(h.engine.queue(), &texture_a, &seed);
        let view_a = texture_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = texture_b.create_view(&wgpu::TextureViewDescriptor::default());
        let compute_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc67_reaction_compute_layout"),
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
        let bind_a_to_b = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc67_a_to_b"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view_b),
                    },
                ],
            });
        let bind_b_to_a = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc67_b_to_a"),
                layout: &compute_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view_b),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view_a),
                    },
                ],
            });
        let bind_a_to_b_handle = h.insert_bind_group(bind_a_to_b, 1);
        let bind_b_to_a_handle = h.insert_bind_group(bind_b_to_a, 1);
        let compute_pipeline =
            h.register_compute_pipeline("compute_reaction_diffusion.wgsl", &[&compute_layout]);
        let render_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("tc67_reaction_render_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                });
        let render_bg = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("tc67_reaction_render_bg"),
                layout: &render_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view_a),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&h.sampler),
                    },
                ],
            });
        let render_bg_handle = h.insert_bind_group(render_bg, 1);
        let shader_code =
            fs::read_to_string("tests/shared_assets/shaders/render_reaction_diffusion.wgsl")
                .unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("tc67_render_reaction_diffusion"),
                source: wgpu::ShaderSource::Wgsl(shader_code.into()),
            });
        let pipeline_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("tc67_reaction_render_pipeline_layout"),
                    bind_group_layouts: &[Some(&render_layout)],
                    immediate_size: 0,
                });
        let render_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("tc67_reaction_render_pipeline"),
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
                            blend: None,
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
        let (target_handle, target_texture) = h.create_target("TC67 Reaction Diffusion Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        let mut compute_commands = Vec::with_capacity(2480);
        for _ in 0..1240 {
            compute_commands.push(
                ComputeCommand::new(compute_pipeline, [50, 38, 1]).with_bind_group(
                    0,
                    bind_a_to_b_handle,
                    Vec::new(),
                ),
            );
            compute_commands.push(
                ComputeCommand::new(compute_pipeline, [50, 38, 1]).with_bind_group(
                    0,
                    bind_b_to_a_handle,
                    Vec::new(),
                ),
            );
        }
        graph.add_compute_batch(&mut pool, compute_commands);
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
        let (cold_ms, cold_bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        upload_seed(h.engine.queue(), &texture_a, &seed);
        let (warm_ms, bytes) = execute_graph(
            &mut h,
            &mut pool,
            &graph,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        assert!(bytes.iter().any(|byte| *byte != 0));
        write_result(
            &h,
            "tc67_pingpong",
            manifest_text,
            &manifest,
            &target_texture,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            cold_ms,
            &cold_bytes,
            warm_ms,
            &bytes,
            serde_json::json!({ "step_count": 2480, "pair_count": 1240, "seed_reset_before_warm": true }),
        );
        let _ = (texture_a_handle, texture_b_handle);
    });
}
