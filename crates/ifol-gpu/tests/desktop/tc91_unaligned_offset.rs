mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;
use wgpu::util::DeviceExt;

const VALID_COUNT: usize = 301; // Unaligned odd count!
const TOTAL_SLOTS: usize = 320; // 5 workgroups * 64 threads = 320 threads
const WORKGROUP_SIZE: u32 = 64;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    valid_count: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

#[test]
fn test_tc91_unaligned_offset() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let mut src_data = vec![0.0f32; TOTAL_SLOTS];
        let mut expected_dst = vec![0.0f32; TOTAL_SLOTS];

        for i in 0..VALID_COUNT {
            let val = (i as f32) * 0.5 + 1.0;
            src_data[i] = val;
            expected_dst[i] = val * 3.0 + 0.5;
        }

        let initial_dst = vec![0.0f32; TOTAL_SLOTS];

        let (buf_src_h, _) = h.create_storage_buffer(&src_data, "Unaligned Src Buffer", wgpu::BufferUsages::empty());
        let (buf_dst_h, buf_dst) = h.create_storage_buffer(&initial_dst, "Unaligned Dst Buffer", wgpu::BufferUsages::STORAGE);

        let params = Params {
            valid_count: VALID_COUNT as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };
        let param_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Unaligned Params Uniform"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // 1. Compute Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_unaligned_bgl"),
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

        let compute_pipe_h = h.register_compute_pipeline("compute_unaligned.wgsl", &[&compute_bgl]);

        let compute_bg = {
            let raw_src = h.registry.buffer(&buf_src_h).unwrap();
            let raw_dst = h.registry.buffer(&buf_dst_h).unwrap();

            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute_unaligned_bg"),
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_src.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: raw_dst.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: param_buf.as_entire_binding() },
                ],
            })
        };
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        // 2. Render Pipeline (Bar Chart Visualizer)
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_unaligned_bgl"),
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
            let raw_dst = h.registry.buffer(&buf_dst_h).unwrap();
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("render_unaligned_bg"),
                layout: &render_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_dst.as_entire_binding() },
                ],
            })
        };
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests").join("shared_assets").join("shaders").join("render_unaligned.wgsl");
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_unaligned.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_unaligned_layout"),
            bind_group_layouts: &[Some(&render_bgl)],
            immediate_size: 0,
        });
        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_unaligned_pipeline"),
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

        // 3. Build Graph
        let workgroups = ((VALID_COUNT as u32) + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE; // 5 workgroups
        let mut pool = RenderNodePool::new();
        let (target_h, target_tex) = h.create_target("tc91_target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_h,
            width: 800,
            height: 600,
        }).with_clear_color([0.02, 0.04, 0.06, 1.0]);

        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(compute_pipe_h, [workgroups, 1, 1])
                .with_bind_group(0, compute_bg_h, Vec::new()),
        ]);

        graph.add_batch(&mut pool, vec![
            DrawCommand::new(render_pipe_h, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        let start_time = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Unaligned compute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub),
            timeout: None,
        });
        let exec_time = start_time.elapsed();

        // Verify Readback
        let actual_dst: Vec<f32> = h.readback_storage_buffer(&buf_dst, TOTAL_SLOTS);
        assert_eq!(actual_dst.len(), TOTAL_SLOTS);

        let mut matched_valid = 0;
        for i in 0..VALID_COUNT {
            if (actual_dst[i] - expected_dst[i]).abs() < 1e-4 {
                matched_valid += 1;
            }
        }

        let mut untouched_padding = 0;
        for i in VALID_COUNT..TOTAL_SLOTS {
            if actual_dst[i] == 0.0 {
                untouched_padding += 1;
            }
        }

        assert_eq!(matched_valid, VALID_COUNT, "All 301 unaligned valid items must match!");
        assert_eq!(untouched_padding, TOTAL_SLOTS - VALID_COUNT, "All 19 padding slots must remain untouched 0.0!");

        // Save Output & Report
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc91_unaligned_offset.png");

        let actual_rendered_tex = h.registry.owned_texture(&target_h).unwrap_or(&target_tex);
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).unwrap();

        let reports_dir = std::path::Path::new(manifest_dir).join("tests").join("reports");
        std::fs::create_dir_all(&reports_dir).unwrap();
        let report_path = reports_dir.join("tc91_unaligned_offset_report.md");

        let report_content = format!(
r#"# Báo cáo: TC91_UNALIGNED_OFFSET - Unaligned Workgroup & Boundary Guarding Safety

Đây là báo cáo tổng hợp chi tiết kết quả kiểm thử an toàn bộ nhớ VRAM khi xử lý mảng phần tử lẻ không chia hết cho kích thước Workgroup (`workgroup_size(64)`) của bài test TC91.

---

## 1. Môi trường & Thông số Thực thi Desktop (Tauri/wgpu)

- **Cấu hình Dispatch:** 5 Workgroups (Tổng 320 luồng GPU đồng thời)
- **Kích thước Mảng Thực tế:** 301 phần tử `f32` (Phần tử lẻ - Unaligned)
- **Kích thước Mảng Bộ Nhớ VRAM Allocated:** 320 phần tử (Bao gồm 19 phần tử Padding)
- **Thời gian Thực thi:** {:.2?}

### Kết quả Ảnh Render (Biểu Đồ Trực Quan):

<img src="../outputs/desktop/tc91_unaligned_offset.png" alt="TC91 Desktop Render" />

- **Giải thích hình ảnh trực quan:**
  - **Dải Cột Xanh Lái/Xanh Lam (301 cột):** Phản ánh 301 phần tử dữ liệu hợp lệ được Compute Shader tính toán chính xác $100\%$ theo công thức $Y = 3.0X + 0.5$.
  - **Vạch Đỏ Ranh Giới (Red Guard Line):** Vạch chặn biên đứng tại vị trí index 301.
  - **Vùng Đáy Sau Vạch Đỏ (19 slots):** 19 luồng GPU dư thừa bị ngắt biên bởi `if (idx >= valid_count) return;`, do đó giữ nguyên mức 0 (không có cột đỏ ghi đè).

---

## 2. Xác Thực Số Học Readback CPU

- **301/301 phần tử valid:** Khớp chính xác $100\%$.
- **19/19 phần tử padding:** Giữ nguyên giá trị `0.0` tuyệt đối.
- **Trạng thái:** **PASSED (An toàn bộ nhớ Boundary Protection 100%)**
"#,
            exec_time
        );

        std::fs::write(&report_path, report_content).unwrap();
        println!("TC91 Unaligned Offset completed successfully! Visual Bar Chart rendered.");
    });
}
