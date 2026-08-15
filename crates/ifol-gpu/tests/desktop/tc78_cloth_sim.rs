use std::time::Instant;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    pos: [f32; 4],
    old_pos: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeParams {
    time: f32,
    delta_time: f32,
    grid_size: u32,
    pad: u32,
}

#[test]
fn test_tc78_cloth_sim() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc78_target");

        let size = 16;
        let mut particles = Vec::new();
        
        for y in 0..size {
            for x in 0..size {
                let px = (x as f32 / (size - 1) as f32) * 1.8 - 0.4;
                let py = 1.0 - (y as f32 / (size - 1) as f32) * 1.8;
                
                // Slightly bend the old position to give initial velocity
                particles.push(Particle {
                    pos: [px, py, 0.0, 1.0],
                    old_pos: [px, py, 0.0, 1.0],
                });
            }
        }
        
        let (_, particle_buf) = h.create_storage_buffer(&particles, "particles", wgpu::BufferUsages::empty());
        
        let params = ComputeParams { time: 5.0, delta_time: 1.0, grid_size: 16, pad: 0 };
        let (_, param_buf) = h.create_storage_buffer(&[params], "params", wgpu::BufferUsages::UNIFORM);

        // Compute Pipeline
        let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_cloth_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let compute_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_cloth_bg"), layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &particle_buf, offset: 0, size: None }) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &param_buf, offset: 0, size: None }) },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bg, 1);

        let compute_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_cloth.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_cloth.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_code)),
        });

        let compute_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cs_cloth_layout"), bind_group_layouts: &[Some(&compute_bgl)], immediate_size: 0,
        });

        let compute_pipe = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cs_cloth"), layout: Some(&compute_layout), module: &compute_shader, entry_point: Some("cs_main"), compilation_options: Default::default(), cache: None,
        });
        let compute_pipe_h = h.insert_compute_pipeline(compute_pipe, vec![Some(1)]);

        // Render Pipeline
        let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render_cloth_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let render_bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render_cloth_bg"), layout: &render_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &particle_buf, offset: 0, size: None }) },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bg, 2);

        let render_code = std::fs::read_to_string("tests/shared_assets/shaders/render_cloth.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_cloth.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_code)),
        });

        let render_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render_cloth_layout"), bind_group_layouts: &[Some(&render_bgl)], immediate_size: 0,
        });

        let render_pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_cloth"), layout: Some(&render_layout),
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
        
        // Dispatch Compute (Simulation step)
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(compute_pipe_h, [1, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
        ]);

        // Draw Pass
        let num_triangles = (size - 1) * (size - 1) * 2;
        let num_vertices = num_triangles * 3;
        
        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural { vertex_count: num_vertices, instance_range: 0..1 },
        ).with_bind_group(0, render_bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("Compute Cloth Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc78_cloth_sim", "Compute Cloth Simulation", "Verlet integration and relaxation of 16x16 cloth grid inside a single Compute Workgroup", "Render output");
    });
}
