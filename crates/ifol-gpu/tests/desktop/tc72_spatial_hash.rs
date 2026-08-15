use std::time::Instant;
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use wgpu::util::DeviceExt;

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Particle {
    pos: [f32; 2],
    vel: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Params {
    num_particles: u32,
    grid_size: u32,
    cell_size: f32,
    radius: f32,
    dt: f32,
    _pad: [f32; 3],
}

#[test]
fn test_tc72_spatial_hash() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 800).await;
        let (target_handle, target_tex) = h.create_target("tc72_target");

        let num_particles = 4096;
        let grid_size = 32;
        let cell_size = 25.0;
        
        // Init particles
        let mut particles = vec![Particle { pos: [0.0; 2], vel: [0.0; 2], color: [1.0; 4] }; num_particles as usize];
        for i in 0..num_particles {
            let x = (i % 64) as f32 * 12.0 + 10.0;
            let y = (i / 64) as f32 * 12.0 + 10.0;
            particles[i as usize] = Particle {
                pos: [x, y],
                vel: [((i % 3) as f32 - 1.0) * 10.0, ((i % 5) as f32 - 2.0) * 10.0],
                color: [1.0, 1.0, 1.0, 1.0],
            };
        }

        let (_particle_buf_h, particle_buf) = h.create_storage_buffer(&particles, "particle_buffer", wgpu::BufferUsages::VERTEX);

        let grid_cells = grid_size * grid_size;
        let grid_cell_size = 144; // 4 (count) + 128 (particles) + 12 (pad)
        let grid_bytes = vec![0u8; (grid_cells * grid_cell_size) as usize];
        let (_grid_buf_h, grid_buf) = h.create_storage_buffer(&grid_bytes, "grid_buffer", wgpu::BufferUsages::empty());

    let params = Params {
        num_particles,
        grid_size,
        cell_size,
        radius: 4.0,
        dt: 0.16, // run simulation step 16ms
        _pad: [0.0; 3],
    };
    
    let uniform_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let compute_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("compute_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
        ],
    });

    let compute_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("compute_bg"),
        layout: &compute_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &particle_buf, offset: 0, size: None }) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &grid_buf, offset: 0, size: None }) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform_buf, offset: 0, size: None }) },
        ],
    });
    let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

    let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_spatial_hash.wgsl").unwrap();
    let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compute_spatial_hash.wgsl"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&compute_shader_code)),
    });
    let compute_pipeline_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("compute_layout"),
        bind_group_layouts: &[Some(&compute_bgl)],
        immediate_size: 0,
    });
    let p_reset = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cs_reset_grid"), layout: Some(&compute_pipeline_layout), module: &compute_shader, entry_point: Some("cs_reset_grid"), compilation_options: Default::default(), cache: None,
    });
    let p_hash = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cs_hash_particles"), layout: Some(&compute_pipeline_layout), module: &compute_shader, entry_point: Some("cs_hash_particles"), compilation_options: Default::default(), cache: None,
    });
    let p_sim = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("cs_simulate"), layout: Some(&compute_pipeline_layout), module: &compute_shader, entry_point: Some("cs_simulate"), compilation_options: Default::default(), cache: None,
    });
    let p_reset_h = h.insert_compute_pipeline(p_reset, vec![Some(1)]);
    let p_hash_h = h.insert_compute_pipeline(p_hash, vec![Some(1)]);
    let p_sim_h = h.insert_compute_pipeline(p_sim, vec![Some(1)]);

    // Render pipeline
    let render_bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("render_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            },
        ],
    });
    let render_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render_bg"),
        layout: &render_bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &particle_buf, offset: 0, size: None }) },
        ],
    });
    let render_bg_h = h.insert_bind_group(render_bind_group, 2);

    let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_spatial_hash.wgsl").unwrap();
    let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("render_spatial_hash.wgsl"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&render_shader_code)),
    });
    let render_pipeline_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render_layout"),
        bind_group_layouts: &[Some(&render_bgl)],
        immediate_size: 0,
    });
    let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render_spatial_hash"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState { module: &render_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module: &render_shader, entry_point: Some("fs_main"),
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

    // Create graph and add passes
    let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 800 });

    let wg_grid = (grid_size * grid_size + 63) / 64;
    let wg_particles = (num_particles + 63) / 64;

    // We simulate 10 frames to let particles collide a bit
    for _ in 0..10 {
        graph.add_compute_batch(&mut h.pool, vec![
            ComputeCommand::new(p_reset_h, [wg_grid, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
            ComputeCommand::new(p_hash_h, [wg_particles, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
            ComputeCommand::new(p_sim_h, [wg_particles, 1, 1]).with_bind_group(0, compute_bg_h, vec![]),
        ]);
    }

    let draw_cmd = DrawCommand::new(
        render_pipe_h,
        DrawAction::Procedural {
            vertex_count: 6,
            instance_range: 0..num_particles,
        },
    ).with_bind_group(0, render_bg_h, Vec::new());
    
    graph.add_batch(&mut h.pool, vec![draw_cmd]);

    let t_start = Instant::now();
    let sub = h.executor.execute(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
    h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
    let t_elapsed = t_start.elapsed();
    println!("Spatial Hash Compute+Render Time: {:?}", t_elapsed);

    h.execute_and_record(&graph, &target_tex, "tc72_spatial_hash", "GPU Spatial Hashing & Collision", "Particles colliding and staying inside 800x800", "Render output");
    });
}
