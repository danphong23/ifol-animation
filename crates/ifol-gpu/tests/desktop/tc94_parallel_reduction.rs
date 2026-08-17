mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

const ELEMENT_COUNT: usize = 1_000_000;
const WORKGROUP_SIZE: u32 = 256;

#[test]
fn test_tc94_parallel_reduction() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Generate 1,000,000 floats on CPU host, with 1 special MAX value at index 543,210
        let mut input_data = vec![0.0f32; ELEMENT_COUNT];
        for i in 0..ELEMENT_COUNT {
            input_data[i] = ((i % 1000) as f32) * 0.1;
        }
        input_data[543_210] = 9999.5; // Expected MAX value!

        let workgroups = ((ELEMENT_COUNT as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE; // 3907 workgroups
        let output_data = vec![0.0f32; workgroups as usize];

        // 2. Allocate Buffers
        let (buf_in_h, _) = h.create_storage_buffer(&input_data, "Reduction Input Buffer", wgpu::BufferUsages::STORAGE);
        let (buf_out_h, buf_out) = h.create_storage_buffer(&output_data, "Reduction Output Buffer", wgpu::BufferUsages::STORAGE);

        // 3. Compute Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_reduction_bgl"),
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
            ],
        });

        let compute_pipe_h = h.register_compute_pipeline("compute_reduction.wgsl", &[&compute_bgl]);

        let compute_bg = {
            let raw_in = h.registry.buffer(&buf_in_h).unwrap();
            let raw_out = h.registry.buffer(&buf_out_h).unwrap();
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute_reduction_bg"),
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_in.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: raw_out.as_entire_binding() },
                ],
            })
        };
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        // 4. Render Pipeline (Visual Radar Target Star Visualizer)
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_reduction_bgl"),
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
            ],
        });

        let render_bg = {
            let raw_in = h.registry.buffer(&buf_in_h).unwrap();
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("render_reduction_bg"),
                layout: &render_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_in.as_entire_binding() },
                ],
            })
        };
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests").join("shared_assets").join("shaders").join("render_reduction.wgsl");
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_reduction.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_reduction_layout"),
            bind_group_layouts: &[Some(&render_bgl)],
            immediate_size: 0,
        });
        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_reduction_pipeline"),
            layout: Some(&render_layout),
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
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![Some(2)]);

        // 5. Build Graph
        let (target_h, target_tex) = h.create_target("tc94_target");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_h,
            width: 800,
            height: 600,
        }).with_clear_color([0.02, 0.03, 0.05, 1.0]);

        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(compute_pipe_h, [workgroups, 1, 1])
                .with_bind_group(0, compute_bg_h, Vec::new()),
        ]);

        graph.add_batch(&mut pool, vec![
            DrawCommand::new(render_pipe_h, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        let start_time = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Compute reduction execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub),
            timeout: None,
        });
        let exec_time = start_time.elapsed();

        // 6. Final CPU Pass over Reduced Workgroup Array (3,907 elements)
        let partial_max: Vec<f32> = h.readback_storage_buffer(&buf_out, workgroups as usize);
        let mut global_max = -f32::MAX;
        for &val in &partial_max {
            if val > global_max {
                global_max = val;
            }
        }

        assert_eq!(global_max, 9999.5, "GPU Parallel Reduction must find exact global max value 9999.5!");

        // Save Output & Report
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc94_parallel_reduction.png");

        let actual_rendered_tex = h.registry.owned_texture(&target_h).unwrap_or(&target_tex);
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).unwrap();

        let reports_dir = std::path::Path::new(manifest_dir).join("tests").join("reports");
        std::fs::create_dir_all(&reports_dir).unwrap();
        let report_path = reports_dir.join("tc94_parallel_reduction_report.md");

        let report_content = format!(
r#"# Báo cáo: TC94_PARALLEL_REDUCTION - GPU Tree Reduction for 1M Elements

Đây là báo cáo tổng hợp chi tiết kỹ thuật bài kiểm thử **Thuật toán Parallel Tree Reduction (Tìm giá trị Max/Min song song)** trên mảng 1.000.000 phần tử cho TC94.

---

## 1. Môi trường & Thông số Thực thi Desktop (Tauri/wgpu)

- **Kích thước Mảng Dữ Liệu:** 1,000,000 phần tử `f32` (4 MB VRAM Storage Buffer)
- **Vị trí Phần tử Max Đặc Biệt:** Index `543,210` có giá trị `9999.5` (Các phần tử còn lại nằm trong dải `0.0` đến `99.9`)
- **Cấu hình Workgroup Compute:** 256 threads / workgroup (3,907 Workgroups)
- **Thời gian Thực thi:** {:.2?}

### Kết quả Ảnh Render (Radar Target Visualizer):

<img src="../outputs/desktop/tc94_parallel_reduction.png" alt="TC94 Desktop Render" />

- **Giải thích hình ảnh trực quan:**
  - **Tia Định Vị Laser Vàng (Gold Radar Beam):** Quét đứng chính xác tại vị trí index `543,210` (Tỷ lệ $54.32\%$ chiều ngang màn hình).
  - **Ngôi Sao Vàng Phát Sáng (Target Star Glow):** Vị trí đỉnh cao nhất đánh dấu phần tử `Max = 9999.5` được thuật toán GPU Reduction tìm ra.
  - **Dải Sóng Xanh Lục Đáy (Point Cloud Base):** 999,999 phần tử nền còn lại nằm ở mức thấp bên dưới.

---

## 2. Xác Thực Số Học Readback CPU

- **Giá trị Max toàn cục tìm được trên GPU:** **9999.5** (Kỳ vọng: 9999.5).
- **Tỷ lệ khớp:** **100.0%**.
- **Trạng thái:** **PASSED (Xác thực Thuật toán Parallel Tree Reduction thành công 100%)**
"#,
            exec_time
        );

        std::fs::write(&report_path, report_content).unwrap();
        println!("TC94 Parallel Reduction completed successfully! Visual Radar Target Star rendered.");
    });
}
