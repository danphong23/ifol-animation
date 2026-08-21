use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurParams {
    dir: [f32; 2],
    radius: i32,
    pad: i32,
}

#[test]
fn test_tc81_separable_blur() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc81_target");

        let src_tex_info = h.load_texture("bg_nightsky.jpeg");
        
        // Storage Textures for intermediate and final
        let (_inter_h, inter_tex) = h.create_storage_texture(src_tex_info.width, src_tex_info.height, wgpu::TextureFormat::Rgba8Unorm, "intermediate_tex");
        let (_final_h, final_tex) = h.create_storage_texture(src_tex_info.width, src_tex_info.height, wgpu::TextureFormat::Rgba8Unorm, "final_tex");

        let inter_view = inter_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let final_view = final_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let radius = 30; // Huge radius to test performance

        let h_params = BlurParams { dir: [1.0, 0.0], radius, pad: 0 };
        let v_params = BlurParams { dir: [0.0, 1.0], radius, pad: 0 };
        let (_, h_buf) = h.create_storage_buffer(&[h_params], "h_params", wgpu::BufferUsages::UNIFORM);
        let (_, v_buf) = h.create_storage_buffer(&[v_params], "v_params", wgpu::BufferUsages::UNIFORM);

        let (src_view, _) = h.registry.texture(&src_tex_info.handle).unwrap();

        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_blur_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        // Pass 1: SRC -> INTERMEDIATE (Horizontal)
        let bg_hpass = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_hpass"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(src_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&inter_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &h_buf, offset: 0, size: None }) },
            ],
        });
        let bg_hpass_handle = h.insert_bind_group(bg_hpass, 1);

        // Pass 2: INTERMEDIATE -> FINAL (Vertical)
        let bg_vpass = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_vpass"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&inter_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&final_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &v_buf, offset: 0, size: None }) },
            ],
        });
        let bg_vpass_handle = h.insert_bind_group(bg_vpass, 1);

        let compute_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_blur.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_blur.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cs_blur_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let compute_pipe = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_blur"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let compute_pipe_h = h.insert_compute_pipeline(compute_pipe, vec![Some(1)]);

        // Render Pipeline
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_quad_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });

        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_quad_bg"), layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&final_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&h.sampler) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 3);

        let render_code = std::fs::read_to_string("tests/shared_assets/shaders/render_quad.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_quad.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_code)),
        });

        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_quad_layout"), bind_group_layouts: &[Some(&render_bgl)], immediate_size: 0,
        });

        let render_pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_quad"), layout: Some(&render_layout),
            vertex: wgpu::VertexState { module: &render_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipe, vec![Some(3)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        let dispatch_x = src_tex_info.width.div_ceil(16);
        let dispatch_y = src_tex_info.height.div_ceil(16);

        // Dispatch Compute H
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [dispatch_x, dispatch_y, 1]).with_bind_group(0, bg_hpass_handle, vec![]),
        ]);
        // Dispatch Compute V
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [dispatch_x, dispatch_y, 1]).with_bind_group(0, bg_vpass_handle, vec![]),
        ]);

        // Draw Pass
        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Separable Blur Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc81_separable_blur", "Compute Separable Gaussian Blur", "Applying radius 30 Gaussian Blur using 2-pass Compute Shader", "Render output");
    });
}
