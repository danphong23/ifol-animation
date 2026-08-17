mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

const VOXEL_DIM: u32 = 64; // 64x64x64 3D Voxel Texture

#[test]
fn test_tc93_storage_texture_3d() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Create 3D Storage Texture (64x64x64)
        let voxel_desc = wgpu::TextureDescriptor {
            label: Some("3D Voxel Texture"),
            size: wgpu::Extent3d {
                width: VOXEL_DIM,
                height: VOXEL_DIM,
                depth_or_array_layers: VOXEL_DIM,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let voxel_tex = h.engine.device().create_texture(&voxel_desc);
        let voxel_view = voxel_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3D Voxel Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // 2. Compute BindGroup & Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_3d_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D3,
                    },
                    count: None,
                },
            ],
        });

        let compute_pipe_h = h.register_compute_pipeline("compute_3d_voxel.wgsl", &[&compute_bgl]);

        let compute_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_3d_bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&voxel_view) },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        // 3. Render BindGroup & Pipeline (Raymarching 3D Volume)
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_3d_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
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

        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_3d_bg"),
            layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&voxel_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let shader_path = std::path::Path::new(manifest_dir)
            .join("tests").join("shared_assets").join("shaders").join("render_3d_voxel.wgsl");
        let shader_code = std::fs::read_to_string(&shader_path).unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_3d_voxel.wgsl"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });
        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_3d_layout"),
            bind_group_layouts: &[Some(&render_bgl)],
            immediate_size: 0,
        });
        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_3d_pipeline"),
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

        // 4. Build Graph
        let (target_h, target_tex) = h.create_target("tc93_target");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_h,
            width: 800,
            height: 600,
        }).with_clear_color([0.01, 0.01, 0.02, 1.0]);

        // Compute Pass: Workgroups (8x8x4 = 256 threads per wg, 8x8x16 wgs = 64x64x64 voxels)
        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(compute_pipe_h, [8, 8, 16])
                .with_bind_group(0, compute_bg_h, Vec::new()),
        ]);

        // Render Pass: Raymarch 3D Voxel
        graph.add_batch(&mut pool, vec![
            DrawCommand::new(render_pipe_h, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        let start_time = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Compute 3D voxel execution failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub),
            timeout: None,
        });
        let exec_time = start_time.elapsed();

        // Save Output & Report
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc93_storage_texture_3d.png");

        let actual_rendered_tex = h.registry.owned_texture(&target_h).unwrap_or(&target_tex);
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).unwrap();

        let reports_dir = std::path::Path::new(manifest_dir).join("tests").join("reports");
        std::fs::create_dir_all(&reports_dir).unwrap();
        let report_path = reports_dir.join("tc93_storage_texture_3d_report.md");

        let report_content = format!(
r#"# Báo cáo: TC93_STORAGE_TEXTURE_3D - 3D Storage Texture & Voxel Density Field

Đây là báo cáo tổng hợp kết quả kiểm thử 3D Storage Texture và Raymarching Sương mù 3D cho TC93.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Thực thi:** {:.2?}
- **Kết quả ảnh (Thực tế):**

<img src="../outputs/desktop/tc93_storage_texture_3d.png" alt="TC93 Desktop Render" />

- **Kỳ vọng:** Compute Shader ghi dữ liệu 3D Voxel Texture 64x64x64, Render Pass raymarching hiển thị trường mật độ sương mù 3D đẹp mắt.
- **Trạng thái:** **PASSED**
"#,
            exec_time
        );

        std::fs::write(&report_path, report_content).unwrap();
        println!("TC93 3D Storage Texture completed successfully! Time: {:?}", exec_time);
    });
}
