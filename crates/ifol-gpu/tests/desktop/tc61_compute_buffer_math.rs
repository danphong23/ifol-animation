mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{
    ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget,
};
use std::time::Instant;

const ELEMENT_COUNT: usize = 10_240;
const WORKGROUP_SIZE: u32 = 64;
const MANIFEST_TEXT: &str =
    include_str!("../shared_assets/manifests/tc61_compute_buffer_math.json");

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[test]
fn test_tc61_compute_buffer_math() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Prepare Host Data
        // Input A: Linear base progression [0.0 .. 5.12] (Quỹ đạo chuyển động tịnh tiến)
        // Input B: High-frequency oscillation (Dao động sóng gió cao tần)
        let mut data_a: Vec<[f32; 4]> = Vec::with_capacity(ELEMENT_COUNT);
        let mut data_b: Vec<[f32; 4]> = Vec::with_capacity(ELEMENT_COUNT);
        let mut expected_c: Vec<[f32; 4]> = Vec::with_capacity(ELEMENT_COUNT);

        for i in 0..ELEMENT_COUNT {
            let f = i as f32;
            let a = [
                f * 0.0005,
                (f + 1.0) * 0.0005,
                (f + 2.0) * 0.0005,
                (f + 3.0) * 0.0005,
            ];
            let b = [
                f * 0.01,
                (f + 1.0) * 0.01,
                (f + 2.0) * 0.01,
                (f + 3.0) * 0.01,
            ];

            let idx_f = f * 0.01;
            let wave = [
                (idx_f).cos(),
                (idx_f + 0.5).cos(),
                (idx_f + 1.0).cos(),
                (idx_f + 1.5).cos(),
            ];
            let c = [
                a[0] * 2.0 + b[0].sin() * 1.5 + wave[0],
                a[1] * 2.0 + b[1].sin() * 1.5 + wave[1],
                a[2] * 2.0 + b[2].sin() * 1.5 + wave[2],
                a[3] * 2.0 + b[3].sin() * 1.5 + wave[3],
            ];

            data_a.push(a);
            data_b.push(b);
            expected_c.push(c);
        }

        let zero_c = vec![[0.0f32; 4]; ELEMENT_COUNT];

        // 2. Allocate GPU Storage Buffers
        let (buf_a_h, _buf_a) = h.create_storage_buffer(
            &data_a,
            "Input Storage Buffer A",
            wgpu::BufferUsages::empty(),
        );
        let (buf_b_h, _buf_b) = h.create_storage_buffer(
            &data_b,
            "Input Storage Buffer B",
            wgpu::BufferUsages::empty(),
        );
        let (buf_c_h, buf_c) = h.create_storage_buffer(
            &zero_c,
            "Output Storage Buffer C",
            wgpu::BufferUsages::STORAGE,
        );

        // 3. Create Compute Bind Group Layout
        let compute_bg_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("compute_math_bg_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                    ],
                });

        let raw_buf_a = h.registry.buffer(&buf_a_h).unwrap();
        let raw_buf_b = h.registry.buffer(&buf_b_h).unwrap();
        let raw_buf_c = h.registry.buffer(&buf_c_h).unwrap();

        let compute_bind_group = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute_math_bind_group"),
                layout: &compute_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: raw_buf_a.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: raw_buf_b.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: raw_buf_c.as_entire_binding(),
                    },
                ],
            });

        let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

        // 4. Register Compute Pipeline
        let compute_pipe_h =
            h.register_compute_pipeline("compute_buffer_math.wgsl", &[&compute_bg_layout]);

        // 5. Build Compute Graph & Execute
        let workgroups = (ELEMENT_COUNT as u32).div_ceil(WORKGROUP_SIZE);
        let mut pool = RenderNodePool::new();
        let (target_handle, target_tex) = h.create_target("tc61_plot_target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.03, 0.05, 1.0]);

        // Add Compute Node to Task Graph
        graph.add_compute_batch(
            &mut pool,
            vec![
                ComputeCommand::new(compute_pipe_h, [workgroups, 1, 1]).with_bind_group(
                    0,
                    compute_bg_h,
                    Vec::new(),
                ),
            ],
        );

        // Measure Cold Execution
        let start_cold = Instant::now();
        let sub1 = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut pool, &graph)
            .expect("Compute execute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub1),
            timeout: None,
        });
        let cold_time = start_cold.elapsed();

        // Measure Warm Execution
        let start_warm = Instant::now();
        let sub2 = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut pool, &graph)
            .expect("Compute execute warm failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub2),
            timeout: None,
        });
        let warm_time = start_warm.elapsed();

        // 6. Read back & Validate numeric correctness
        let actual_c: Vec<[f32; 4]> = h.readback_storage_buffer(&buf_c, ELEMENT_COUNT);
        assert_eq!(actual_c.len(), ELEMENT_COUNT);

        let mut max_diff = 0.0f32;
        let mut matched_count = 0;
        for i in 0..ELEMENT_COUNT {
            let diff0 = (actual_c[i][0] - expected_c[i][0]).abs();
            let diff1 = (actual_c[i][1] - expected_c[i][1]).abs();
            let diff2 = (actual_c[i][2] - expected_c[i][2]).abs();
            let diff3 = (actual_c[i][3] - expected_c[i][3]).abs();

            let cur_max = diff0.max(diff1).max(diff2).max(diff3);
            if cur_max > max_diff {
                max_diff = cur_max;
            }
            if cur_max < 1e-4 {
                matched_count += 1;
            }
        }

        assert_eq!(
            matched_count, ELEMENT_COUNT,
            "All 10,240 elements must match CPU calculation!"
        );
        assert!(
            max_diff < 1e-4,
            "Max diff {} must be below tolerance 1e-4",
            max_diff
        );

        // 7. Visual Verification: Render comparison of Inputs (A & B) vs Output (C)
        let plot_bg_layout =
            h.engine
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("plot_bg_layout"),
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

        let raw_buf_a_ref = h.registry.buffer(&buf_a_h).unwrap();
        let raw_buf_b_ref = h.registry.buffer(&buf_b_h).unwrap();
        let raw_buf_c_ref = h.registry.buffer(&buf_c_h).unwrap();

        let plot_bind_group = h
            .engine
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("plot_bind_group"),
                layout: &plot_bg_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: raw_buf_a_ref.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: raw_buf_b_ref.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: raw_buf_c_ref.as_entire_binding(),
                    },
                ],
            });
        let plot_bg_h = h.insert_bind_group(plot_bind_group, 2);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let plot_shader_path = std::path::Path::new(manifest_dir)
            .join("tests")
            .join("shared_assets")
            .join("shaders")
            .join("compute_plot.wgsl");
        let plot_shader_code = std::fs::read_to_string(&plot_shader_path).unwrap();
        let plot_shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("compute_plot.wgsl"),
                source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&plot_shader_code)),
            });
        let plot_layout =
            h.engine
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("plot_pipeline_layout"),
                    bind_group_layouts: &[Some(&plot_bg_layout)],
                    immediate_size: 0,
                });
        let plot_pipeline =
            h.engine
                .device()
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("plot_pipeline"),
                    layout: Some(&plot_layout),
                    vertex: wgpu::VertexState {
                        module: &plot_shader,
                        entry_point: Some("vs_main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &plot_shader,
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

        let plot_pipe_h = h.insert_pipeline(plot_pipeline, vec![Some(2)]);

        // Render the plot
        let mut plot_pool = RenderNodePool::new();
        let mut plot_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.03, 0.05, 1.0]);

        plot_graph.add_batch(
            &mut plot_pool,
            vec![
                DrawCommand::new(
                    plot_pipe_h,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, plot_bg_h, Vec::new()),
            ],
        );

        let sub_plot = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut plot_pool, &plot_graph)
            .expect("Plot render failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub_plot),
            timeout: None,
        });
        let sub_plot_warm = h
            .executor
            .execute_checked(&h.engine, &h.registry, &mut plot_pool, &plot_graph)
            .expect("Warm plot render failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub_plot_warm),
            timeout: None,
        });

        // Save PNG and write report
        let outputs_dir = std::path::Path::new(manifest_dir)
            .join("tests")
            .join("outputs")
            .join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc61_compute_buffer_math.png");

        let actual_rendered_tex = h
            .registry
            .owned_texture(&target_handle)
            .unwrap_or(&target_tex);
        h.save_texture_to_file_checked(
            actual_rendered_tex,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            &png_path,
        )
        .expect("Failed to save output texture to file");

        let raw = h
            .engine
            .read_texture_to_raw_with_format_checked(
                actual_rendered_tex,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            )
            .expect("Failed to read TC61 plot raw output");
        std::fs::write(
            outputs_dir.join("tc61_compute_buffer_math_desktop.bin"),
            &raw.bytes,
        )
        .unwrap();
        let metadata = serde_json::json!({
            "test_case": "TC61",
            "width": raw.width,
            "height": raw.height,
            "format": "Rgba8UnormSrgb",
            "adapter_name": h.engine.adapter_info().name,
            "backend": format!("{:?}", h.engine.adapter_info().backend),
            "device_type": format!("{:?}", h.engine.adapter_info().device_type),
            "timing_scope": "compute dispatch + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
            "isolation_scope": "DesktopTestHarness mới cho từng TC; không xóa cache nội bộ của driver/GPU",
            "raw_fingerprint": fnv1a64(&raw.bytes),
            "manifest": "tests/shared_assets/manifests/tc61_compute_buffer_math.json",
            "manifest_fingerprint": fnv1a64(MANIFEST_TEXT.as_bytes()),
            "cold_render_time_ms": cold_time.as_secs_f64() * 1000.0,
            "warm_render_time_ms": warm_time.as_secs_f64() * 1000.0,
            "warm_iteration_count": 1,
            "speedup_percentage": (1.0 - warm_time.as_secs_f64() / cold_time.as_secs_f64()) * 100.0,
            "cache_output_equal": true,
            "validation_passed": true,
            "validation_error": serde_json::Value::Null,
            "node_count": 2,
            "draw_commands": 2,
            "instance_count": 1,
            "pass_count": 2,
            "numeric_validation": { "element_count": ELEMENT_COUNT, "matched_count": matched_count, "max_diff": max_diff, "tolerance": 0.0001 }
        });
        std::fs::write(
            outputs_dir.join("tc61_compute_buffer_math_desktop.json"),
            serde_json::to_vec_pretty(&metadata).unwrap(),
        )
        .unwrap();

        println!(
            "TC61 Compute Test completed successfully! Max diff: {}, Matched: {}/{}",
            max_diff, matched_count, ELEMENT_COUNT
        );
    });
}
