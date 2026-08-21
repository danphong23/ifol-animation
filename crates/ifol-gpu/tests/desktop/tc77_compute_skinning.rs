use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeVertex {
    pos: [f32; 4],
    uv: [f32; 2],
    pad: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeParams {
    time: f32,
    count: u32,
    pad1: u32,
    pad2: u32,
}

#[test]
fn test_tc77_compute_skinning() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc77_target");

        let grid_w = 40;
        let grid_h = 20;
        let mut vertices = Vec::new();
        
        for y in 0..grid_h {
            for x in 0..grid_w {
                let u0 = x as f32 / grid_w as f32;
                let v0 = y as f32 / grid_h as f32;
                let u1 = (x + 1) as f32 / grid_w as f32;
                let v1 = (y + 1) as f32 / grid_h as f32;

                let quad = [
                    ComputeVertex { pos: [u0, v0, 0.0, 1.0], uv: [u0, v0], pad: [0.0; 2] },
                    ComputeVertex { pos: [u1, v0, 0.0, 1.0], uv: [u1, v0], pad: [0.0; 2] },
                    ComputeVertex { pos: [u1, v1, 0.0, 1.0], uv: [u1, v1], pad: [0.0; 2] },
                    
                    ComputeVertex { pos: [u0, v0, 0.0, 1.0], uv: [u0, v0], pad: [0.0; 2] },
                    ComputeVertex { pos: [u1, v1, 0.0, 1.0], uv: [u1, v1], pad: [0.0; 2] },
                    ComputeVertex { pos: [u0, v1, 0.0, 1.0], uv: [u0, v1], pad: [0.0; 2] },
                ];
                vertices.extend_from_slice(&quad);
            }
        }
        
        let count = vertices.len() as u32;

        let (_in_buf_h, in_buf) = h.create_storage_buffer(&vertices, "in_vertices", wgpu::BufferUsages::empty());
        let (_out_buf_h, out_buf) = h.create_storage_buffer(&vertices, "out_vertices", wgpu::BufferUsages::VERTEX);
        
        let params = ComputeParams { time: 1.5, count, pad1: 0, pad2: 0 };
        let (_param_buf_h, param_buf) = h.create_storage_buffer(&[params], "params", wgpu::BufferUsages::UNIFORM);

        // Compute Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_skinning_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let compute_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_skinning_bg"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &in_buf, offset: 0, size: None }) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &out_buf, offset: 0, size: None }) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &param_buf, offset: 0, size: None }) },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        let compute_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_skinning.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_skinning.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cs_skinning_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let compute_pipe = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_skinning"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let compute_pipe_h = h.insert_compute_pipeline(compute_pipe, vec![Some(1)]);

        // Render Pipeline
        let render_code = std::fs::read_to_string("tests/shared_assets/shaders/render_skinning.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_skinning.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_code)),
        });

        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_skinning_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_skinning_bg"), layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &out_buf, offset: 0, size: None }) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_skinning_layout"), bind_group_layouts: &[Some(&render_bgl)], immediate_size: 0,
        });

        let render_pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_skinning"), layout: Some(&render_layout),
            vertex: wgpu::VertexState { module: &render_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipe, vec![Some(2)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        let wg_x = count.div_ceil(64);
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [wg_x, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
        ]);

        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: count, instance_range: 0..1 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute_checked(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Compute Skinning Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc77_compute_skinning", "Compute Mesh Skinning", "Transforming vertices in Compute Shader for mesh deformation", "Render output");
    });
}
