mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct IndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    pos: [f32; 2],
    color: [f32; 4],
}

#[test]
fn test_tc92_compute_indirect() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Allocate Indirect Buffer (64 bytes)
        let initial_args = IndirectArgs { vertex_count: 0, instance_count: 0, first_vertex: 0, first_instance: 0 };
        let (buf_indirect_h, buf_indirect) = h.create_storage_buffer(&[initial_args], "Indirect Args Buffer", wgpu::BufferUsages::INDIRECT);

        // 2. Allocate Particle Buffer (1000 particles)
        let dummy_particles = vec![Particle { pos: [0.0, 0.0], color: [0.0, 0.0, 0.0, 0.0] }; 1000];
        let (buf_particles_h, _) = h.create_storage_buffer(&dummy_particles, "Indirect Particles Buffer", wgpu::BufferUsages::STORAGE);

        // Compute BindGroup & Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_indirect_bgl"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_pipe_h = h.register_compute_pipeline("compute_indirect_gen.wgsl", &[&compute_bgl]);

        let compute_bg = {
            let raw_ind = h.registry.buffer(&buf_indirect_h).unwrap();
            let raw_part = h.registry.buffer(&buf_particles_h).unwrap();
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compute_indirect_bg"),
                layout: &compute_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_ind.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: raw_part.as_entire_binding() },
                ],
            })
        };
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        // Render Pipeline
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_indirect_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
            let raw_part = h.registry.buffer(&buf_particles_h).unwrap();
            h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("render_indirect_bg"),
                layout: &render_bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: raw_part.as_entire_binding() },
                ],
            })
        };
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests").join("shared_assets").join("shaders").join("render_indirect.wgsl");
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_indirect.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_indirect_layout"),
            bind_group_layouts: &[Some(&render_bgl)],
            immediate_size: 0,
        });
        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_indirect_pipeline"),
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
        let (target_h, target_tex) = h.create_target("tc92_target");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_h,
            width: 800,
            height: 600,
        }).with_clear_color([0.02, 0.02, 0.05, 1.0]);

        // Compute Pass: Generate Particles & Write Indirect Args
        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(compute_pipe_h, [16, 1, 1])
                .with_bind_group(0, compute_bg_h, Vec::new()),
        ]);

        // Render Pass: Render using Procedural DrawAction
        graph.add_batch(&mut pool, vec![
            DrawCommand::new(render_pipe_h, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1000 })
                .with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        let start_time = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Compute indirect execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub),
            timeout: None,
        });
        let exec_time = start_time.elapsed();

        // 4. Verify GPU Indirect Readback
        let readback_args: Vec<IndirectArgs> = h.readback_storage_buffer(&buf_indirect, 1);
        assert_eq!(readback_args.len(), 1);
        assert_eq!(readback_args[0].vertex_count, 6);
        assert_eq!(readback_args[0].instance_count, 1000);

        // Save Output & Report
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc92_compute_indirect.png");

        let actual_rendered_tex = h.registry.owned_texture(&target_h).unwrap_or(&target_tex);
        h.engine.save_texture_to_file_checked(actual_rendered_tex, &png_path).unwrap();

        let reports_dir = std::path::Path::new(manifest_dir).join("tests").join("reports");
        std::fs::create_dir_all(&reports_dir).unwrap();
        let report_path = reports_dir.join("tc92_compute_indirect_report.md");

        let report_content = format!(
r#"# Báo cáo: TC92_COMPUTE_INDIRECT - Compute-to-Draw Indirect Generation

Đây là báo cáo tổng hợp kết quả sinh DrawIndirect từ Compute Shader cho TC92.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Thực thi:** {:.2?}
- **Kết quả ảnh (Thực tế):**

<img src="../outputs/desktop/tc92_compute_indirect.png" alt="TC92 Desktop Render" />

- **Kỳ vọng:** Compute Shader tính toán và sinh cấu hình `DrawIndirectArgs` (vertex_count=6, instance_count=1000) thẳng trên GPU Buffer.
- **Xác thực số học (Readback):**
  - Indirect vertex_count: {} (Kỳ vọng: 6).
  - Indirect instance_count: {} (Kỳ vọng: 1000).
- **Trạng thái:** **PASSED**
"#,
            exec_time, readback_args[0].vertex_count, readback_args[0].instance_count
        );

        std::fs::write(&report_path, report_content).unwrap();
        println!("TC92 Compute Indirect completed successfully! Time: {:?}", exec_time);
    });
}
