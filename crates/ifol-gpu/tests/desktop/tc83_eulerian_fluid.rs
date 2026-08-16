use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FluidParams {
    time: f32,
    dt: f32,
    width: u32,
    height: u32,
}

#[test]
fn test_tc83_eulerian_fluid() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc83_target");

        let width = 800;
        let height = 600;

        let (_src_h, src_tex) = h.create_storage_texture(width, height, wgpu::TextureFormat::Rgba8Unorm, "fluid_src");
        let (_dst_h, dst_tex) = h.create_storage_texture(width, height, wgpu::TextureFormat::Rgba8Unorm, "fluid_dst");

        let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let dst_view = dst_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let params = FluidParams { time: 3.5, dt: 1.0, width, height };
        let (_, param_buf) = h.create_storage_buffer(&[params], "fluid_params", wgpu::BufferUsages::UNIFORM);

        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fluid_compute_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::StorageTexture { access: wgpu::StorageTextureAccess::WriteOnly, format: wgpu::TextureFormat::Rgba8Unorm, view_dimension: wgpu::TextureViewDimension::D2 }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bg_compute = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fluid_compute_bg"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&src_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&dst_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &param_buf, offset: 0, size: None }) },
            ],
        });
        let bg_compute_h = h.insert_bind_group(bg_compute, 1);

        let compute_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_fluid.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_fluid.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fluid_compute_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let compute_pipe = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_fluid"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let compute_pipe_h = h.insert_compute_pipeline(compute_pipe, vec![Some(1)]);

        // Render Quad Pipeline
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
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&dst_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&h.sampler) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

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
        let render_pipe_h = h.insert_pipeline(render_pipe, vec![Some(2)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width, height });
        
        let dispatch_x = (width + 15) / 16;
        let dispatch_y = (height + 15) / 16;

        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [dispatch_x, dispatch_y, 1]).with_bind_group(0, bg_compute_h, vec![]),
        ]);

        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Eulerian Fluid Sim Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc83_eulerian_fluid", "Eulerian Fluid Simulation", "Simulating 2D fluid velocity field and density advection on Compute Shader", "Render output");
    });
}
