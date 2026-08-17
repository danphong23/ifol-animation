mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    pos: [f32; 2],
    radius: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct IndirectArgs {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    cull_center: [f32; 2],
    cull_radius: f32,
    _pad: f32,
}

#[test]
fn test_tc70_culling() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let in_w = 800;
        let in_h = 600;

        let num_particles = 100_000;
        
        let mut particles = Vec::with_capacity(num_particles);
        for i in 0..num_particles {
            // Random-ish positions in [-1, 1]
            let r1 = ((i * 13) % 1000) as f32 / 1000.0;
            let r2 = ((i * 17) % 1000) as f32 / 1000.0;
            particles.push(Particle {
                pos: [r1 * 2.0 - 1.0, r2 * 2.0 - 1.0],
                radius: 0.005,
                _pad: 0.0,
            });
        }

        let (_src_buf_h, src_buf) = h.create_storage_buffer(&particles, "Src Particles", wgpu::BufferUsages::STORAGE);
        let (_dst_buf_h, dst_buf) = h.create_storage_buffer(&particles, "Dst Particles", wgpu::BufferUsages::STORAGE); // Will hold culled output

        // Indirect buffer
        let indirect_initial = IndirectArgs {
            vertex_count: 6, // 6 vertices per quad
            instance_count: 0, // Will be incremented by compute shader
            first_vertex: 0,
            first_instance: 0,
        };
        let (indirect_handle, _indirect_buf) = h.create_storage_buffer(&[indirect_initial], "Indirect Args", wgpu::BufferUsages::INDIRECT);

        let uniforms = Uniforms {
            cull_center: [0.0, 0.0],
            cull_radius: 0.5,
            _pad: 0.0,
        };
        let (_uniform_buf_h, uniform_buf) = h.create_storage_buffer(&[uniforms], "Cull Uniforms", wgpu::BufferUsages::UNIFORM);

        // 2. Compute Pipeline
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cull_compute_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let indirect_buf_ref = h.registry.buffer(&indirect_handle).unwrap();
        let compute_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cull_compute_bind_group"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dst_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: indirect_buf_ref.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: uniform_buf.as_entire_binding() },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_cull.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_cull.wgsl"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
        });
        let compute_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cull_compute_pipe_layout"),
            bind_group_layouts: &[Some(&compute_bg_layout)],
            immediate_size: 0,
        });

        let cull_pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cull_pipeline"),
            layout: Some(&compute_pipe_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let cull_pipe_h = h.insert_compute_pipeline(cull_pipeline, vec![Some(1)]);

        // 3. Render Pipeline setup
        let render_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cull_render_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let render_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cull_render_bind_group"),
            layout: &render_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: dst_buf.as_entire_binding() },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bind_group, 2);

        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_culled.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_culled.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cull_render_pipe_layout"),
            bind_group_layouts: &[Some(&render_bg_layout)],
            immediate_size: 0,
        });

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cull_render_pipeline"),
            layout: Some(&render_pipe_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[], // No vertex buffers, we read directly from storage buffer based on instance_index
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
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![Some(2)]);

        // 4. Build Graph
        let (target_handle, target_tex) = h.create_target("Cull Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.1, 0.1, 0.1, 1.0]);

        // Compute Pass: Cull particles
        let groups_x = (num_particles as u32 + 63) / 64;
        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(cull_pipe_h, [groups_x, 1, 1])
                .with_bind_group(0, compute_bg_h, Vec::new())
        ]);

        // Render Pass: Draw using Indirect buffer (whose instance_count was filled by Compute)
        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Indirect {
                buffer: indirect_handle,
                offset: 0,
            },
        ).with_bind_group(0, render_bg_h, Vec::new());
        graph.add_batch(&mut pool, vec![draw_cmd]);

        let start = Instant::now();
        let sub1 = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub1),
            timeout: None,
        });
        let elapsed = start.elapsed();

        println!("Cull Compute+Render Time: {:?}", elapsed);

        // 5. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc70_culling.png");
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).expect("Save failed");
    });
}
