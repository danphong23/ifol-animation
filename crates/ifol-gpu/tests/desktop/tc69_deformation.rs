mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    _pad: [f32; 3],
}

#[test]
fn test_tc69_deformation() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let in_w = 800;
        let in_h = 600;

        let grid_size = 64;
        let num_vertices = (grid_size + 1) * (grid_size + 1);
        let num_indices = grid_size * grid_size * 6;

        // 1. Generate Grid Mesh
        let mut initial_vertices = Vec::with_capacity(num_vertices);
        for y in 0..=grid_size {
            for x in 0..=grid_size {
                let tx = x as f32 / grid_size as f32; // 0 to 1
                let ty = y as f32 / grid_size as f32; // 0 to 1
                
                initial_vertices.push(Vertex {
                    pos: [tx * 2.0 - 1.0, ty * 2.0 - 1.0], // -1 to 1
                    uv: [tx, ty],
                    color: [0.5, 0.5, 0.5, 1.0],
                });
            }
        }

        let mut indices = Vec::with_capacity(num_indices);
        for y in 0..grid_size {
            for x in 0..grid_size {
                let tl = y * (grid_size + 1) + x;
                let tr = tl + 1;
                let bl = (y + 1) * (grid_size + 1) + x;
                let br = bl + 1;

                indices.push(tl as u16);
                indices.push(bl as u16);
                indices.push(tr as u16);

                indices.push(tr as u16);
                indices.push(bl as u16);
                indices.push(br as u16);
            }
        }

        // Buffer 1: Source Vertices (Read Only Storage)
        let (_src_buf_h, src_buf) = h.create_storage_buffer(&initial_vertices, "Src Vertices", wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE);

        // Buffer 2: Dest Vertices (Read Write Storage AND Vertex usage!)
        let dest_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Dest Vertices"),
            contents: bytemuck::cast_slice(&initial_vertices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Index Buffer
        let index_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Uniform Buffer
        let uniforms = Uniforms { time: 5.0, _pad: [0.0; 3] }; // Time = 5.0 gives some deformation
        let uniform_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Deform Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2. Compute Pipeline setup
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("deform_compute_bg_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry { // Src
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // Dest
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { // Uniforms
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });

        let compute_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("deform_compute_bind_group"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: src_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: dest_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: uniform_buf.as_entire_binding() },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_deformation.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_deformation.wgsl"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
        });
        let compute_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deform_compute_pipe_layout"),
            bind_group_layouts: &[Some(&compute_bg_layout)],
            immediate_size: 0,
        });

        let deform_pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("deform_pipeline"),
            layout: Some(&compute_pipe_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let deform_pipe_h = h.insert_compute_pipeline(deform_pipeline, vec![Some(1)]);

        // 3. Render Pipeline setup
        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_deformation.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_deformation.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("deform_render_pipe_layout"),
            bind_group_layouts: &[], // No bind groups, just Vertex Buffer
            immediate_size: 0,
        });

        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 }, // pos
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x2 }, // uv
                wgpu::VertexAttribute { offset: 16, shader_location: 2, format: wgpu::VertexFormat::Float32x4 }, // color
            ],
        };

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("deform_render_pipeline"),
            layout: Some(&render_pipe_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(vertex_buffer_layout)],
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
                cull_mode: None, // Disable culling since twisting might flip faces
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![]);

        // 4. Build Graph
        let (target_handle, target_tex) = h.create_target("Deform Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.1, 0.1, 0.1, 1.0]);

        // Compute Pass: Transform vertices
        let groups_x = (num_vertices as u32 + 63) / 64;
        graph.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(deform_pipe_h, [groups_x, 1, 1])
                .with_bind_group(0, compute_bg_h, Vec::new())
        ]);

        let mesh_h = h.insert_mesh(
            dest_buf,
            Some((index_buf, wgpu::IndexFormat::Uint16)),
            num_vertices as u32
        );

        // Render Pass: Draw using transformed vertices
        let draw_cmd = DrawCommand::new(
            render_pipe_h,
            DrawAction::Indexed {
                mesh: mesh_h,
                index_range: 0..(num_indices as u32),
                instance_range: 0..1,
            },
        );
        graph.add_batch(&mut pool, vec![draw_cmd]);

        let start = Instant::now();
        let sub1 = h.executor.execute(&h.engine, &h.registry, &mut pool, &graph).expect("Execute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub1),
            timeout: None,
        });
        let elapsed = start.elapsed();

        println!("Deformation Compute+Render Time: {:?}", elapsed);

        // 5. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc69_deformation.png");
        h.engine.save_texture_to_file_checked(actual_rendered_tex, &png_path).expect("Save failed");
    });
}
