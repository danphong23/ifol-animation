mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Particle {
    pos: [f32; 2],
    depth: f32,
    _pad: f32,
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SortParams {
    j: u32,
    k: u32,
}

#[test]
fn test_tc71_bitonic_sort() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let in_w = 800;
        let in_h = 600;

        let num_particles = 65536; // 2^16
        
        let mut particles = Vec::with_capacity(num_particles);
        for i in 0..num_particles {
            // Random-ish positions in [-1, 1]
            let r1 = ((i * 13) % 1000) as f32 / 1000.0;
            let r2 = ((i * 17) % 1000) as f32 / 1000.0;
            let depth = ((i * 23) % 1000) as f32 / 1000.0;
            
            // Color based on depth to easily visualize sorting
            // Far (depth=1.0) -> Red
            // Near (depth=0.0) -> Blue
            let color = [depth, 0.0, 1.0 - depth, 1.0];
            
            particles.push(Particle {
                pos: [r1 * 2.0 - 1.0, r2 * 2.0 - 1.0],
                depth,
                _pad: 0.0,
                color,
            });
        }

        let (_src_buf_h, src_buf) = h.create_storage_buffer(&particles, "Particles", wgpu::BufferUsages::STORAGE);

        // Uniform buffer for bitonic sort params
        let alignment = h.engine.device().limits().min_uniform_buffer_offset_alignment as usize;
        let mut uniform_data = Vec::new();
        
        // Generate (j, k) pairs
        let mut params_list = Vec::new();
        let mut k = 2;
        while k <= num_particles {
            let mut j = k >> 1;
            while j > 0 {
                params_list.push(SortParams { j: j as u32, k: k as u32 });
                j >>= 1;
            }
            k <<= 1;
        }

        for param in &params_list {
            let mut chunk = vec![0u8; alignment];
            let bytes = bytemuck::bytes_of(param);
            chunk[0..bytes.len()].copy_from_slice(bytes);
            uniform_data.extend_from_slice(&chunk);
        }

        let uniform_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sort_params_buf"),
            contents: &uniform_data,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2. Compute Pipeline
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sort_compute_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<SortParams>() as u64) }, count: None },
            ],
        });

        let compute_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sort_compute_bind_group"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform_buf, offset: 0, size: wgpu::BufferSize::new(std::mem::size_of::<SortParams>() as u64) }) },
            ],
        });
        let compute_bg_h = ifol_gpu::resources::BindGroupHandle(999);
        h.registry.insert_bind_group_with_descriptor(
            compute_bg_h,
            compute_bind_group,
            ifol_gpu::resources::BindGroupResourceDescriptor {
                dynamic_offset_count: 1,
                dynamic_offset_alignment: alignment as u32,
                layout_signature: 1,
            },
        ).unwrap();

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_bitonic_sort.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_bitonic_sort.wgsl"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
        });
        let compute_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sort_compute_pipe_layout"),
            bind_group_layouts: &[Some(&compute_bg_layout)],
            immediate_size: 0,
        });

        let sort_pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("sort_pipeline"),
            layout: Some(&compute_pipe_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let sort_pipe_h = h.insert_compute_pipeline(sort_pipeline, vec![Some(1)]);

        // 3. Render Pipeline setup
        let render_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sort_render_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let render_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sort_render_bind_group"),
            layout: &render_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src_buf.as_entire_binding() },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bind_group, 2);

        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_bitonic_sort.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_bitonic_sort.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sort_render_pipe_layout"),
            bind_group_layouts: &[Some(&render_bg_layout)],
            immediate_size: 0,
        });

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sort_render_pipeline"),
            layout: Some(&render_pipe_layout),
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
        let (target_handle, target_tex) = h.create_target("Sort Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.1, 0.1, 0.1, 1.0]);

        // Compute Pass: Sort particles
        let groups_x = (num_particles as u32 + 255) / 256;
        
        let mut compute_cmds = Vec::new();
        for i in 0..params_list.len() {
            let offset = (i * alignment) as u32;
            compute_cmds.push(
                ComputeCommand::new(sort_pipe_h, [groups_x, 1, 1])
                    .with_bind_group(0, compute_bg_h, vec![offset])
            );
        }
        graph.add_compute_batch(&mut pool, compute_cmds);

        // Render Pass: Draw sorted particles
        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Procedural {
                vertex_count: 6,
                instance_range: 0..num_particles as u32,
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

        println!("Bitonic Sort Compute+Render Time: {:?}", elapsed);

        // 5. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc71_bitonic_sort.png");
        h.engine.save_texture_to_file_checked(actual_rendered_tex, &png_path).expect("Save failed");
    });
}
