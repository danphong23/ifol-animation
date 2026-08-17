mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{ComputeCommand, DrawAction, DrawCommand, RenderGraph, RenderNodePool, RenderTarget};
use std::time::Instant;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Node {
    pos: [f32; 2],
    prev_pos: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    _pad: [f32; 3],
}

#[test]
fn test_tc68_verlet() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let in_w = 800;
        let in_h = 600;

        let num_chains = 256;
        let nodes_per_chain = 16;
        let total_nodes = num_chains * nodes_per_chain;

        // 1. Initialize Nodes Data
        let mut initial_nodes = vec![Node { pos: [0.0, 0.0], prev_pos: [0.0, 0.0] }; total_nodes as usize];
        for chain in 0..num_chains {
            let anchor_x = (chain % 16) as f32 * 50.0 + 25.0;
            let anchor_y = (chain / 16) as f32 * 10.0 + 50.0;
            for i in 0..nodes_per_chain {
                let idx = chain * nodes_per_chain + i;
                let y = anchor_y + (i as f32) * 20.0;
                initial_nodes[idx as usize] = Node {
                    pos: [anchor_x, y],
                    prev_pos: [anchor_x, y],
                };
            }
        }

        let (_nodes_buf_h, nodes_buf) = h.create_storage_buffer(&initial_nodes, "Nodes Buffer", wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);

        // 2. Uniform Buffer for time
        let uniforms = Uniforms { time: 5.0, _pad: [0.0; 3] }; // Time = 5.0 gives some sine offset
        let uniform_buf = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Verlet Uniforms"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 3. Compute Bind Group
        let compute_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("verlet_compute_bg_layout"),
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
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("verlet_compute_bind_group"),
            layout: &compute_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: nodes_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: uniform_buf.as_entire_binding() },
            ],
        });
        let compute_bg_h = h.insert_bind_group(compute_bind_group, 1);

        let compute_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/compute_verlet.wgsl").unwrap();
        let compute_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_verlet.wgsl"),
            source: wgpu::ShaderSource::Wgsl(compute_shader_code.into()),
        });
        let compute_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("verlet_compute_pipe_layout"),
            bind_group_layouts: &[Some(&compute_bg_layout)],
            immediate_size: 0,
        });

        // Pass 1: Integrate
        let integrate_pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("integrate_pipeline"),
            layout: Some(&compute_pipe_layout),
            module: &compute_shader,
            entry_point: Some("integrate_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let integrate_pipe_h = h.insert_compute_pipeline(integrate_pipeline, vec![Some(1)]);

        // Pass 2: Constrain
        let constrain_pipeline = h.engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("constrain_pipeline"),
            layout: Some(&compute_pipe_layout),
            module: &compute_shader,
            entry_point: Some("constrain_main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let constrain_pipe_h = h.insert_compute_pipeline(constrain_pipeline, vec![Some(1)]);

        // 4. Render Bind Group
        let render_bg_layout = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("verlet_render_bg_layout"),
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
        let render_bind_group = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("verlet_render_bind_group"),
            layout: &render_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: nodes_buf.as_entire_binding() },
            ],
        });
        let render_bg_h = h.insert_bind_group(render_bind_group, 1);

        let render_shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_chains.wgsl").unwrap();
        let render_shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_chains.wgsl"),
            source: wgpu::ShaderSource::Wgsl(render_shader_code.into()),
        });
        let render_pipe_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("verlet_render_pipe_layout"),
            bind_group_layouts: &[Some(&render_bg_layout)],
            immediate_size: 0,
        });

        let render_pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("verlet_render_pipeline"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Transparent circles
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let render_pipe_h = h.insert_pipeline(render_pipeline, vec![Some(1)]);

        // 5. Build Graph
        let (target_handle, target_tex) = h.create_target("Verlet Output");
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_handle,
            width: in_w,
            height: in_h,
        }).with_clear_color([0.1, 0.1, 0.1, 1.0]);

        // Evolve for 100 frames to see them swing
        let mut compute_commands = Vec::new();
        for _ in 0..100 {
            let groups_int = (total_nodes + 63) / 64;
            compute_commands.push(
                ComputeCommand::new(integrate_pipe_h, [groups_int, 1, 1])
                    .with_bind_group(0, compute_bg_h, Vec::new())
            );
            
            let groups_constrain = (num_chains + 63) / 64;
            compute_commands.push(
                ComputeCommand::new(constrain_pipe_h, [groups_constrain, 1, 1])
                    .with_bind_group(0, compute_bg_h, Vec::new())
            );
        }
        graph.add_compute_batch(&mut pool, compute_commands);

        graph.add_batch(&mut pool, vec![
            DrawCommand::new(
                render_pipe_h,
                DrawAction::Procedural {
                    vertex_count: 6,
                    instance_range: 0..total_nodes,
                },
            ).with_bind_group(0, render_bg_h, Vec::new()),
        ]);

        let start = Instant::now();
        let sub1 = h.executor.execute_checked(&h.engine, &h.registry, &mut pool, &graph).expect("Execute failed");
        let _ = h.engine.device().poll(wgpu::PollType::Wait {
            submission_index: Some(sub1),
            timeout: None,
        });
        let elapsed = start.elapsed();

        println!("Verlet Integration Time (100 frames): {:?}", elapsed);

        // 6. Save Image & Report
        let actual_rendered_tex = h.registry.owned_texture(&target_handle).unwrap_or(&target_tex);
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let outputs_dir = std::path::Path::new(manifest_dir).join("tests").join("outputs").join("desktop");
        std::fs::create_dir_all(&outputs_dir).unwrap();
        let png_path = outputs_dir.join("tc68_verlet.png");
        h.save_texture_to_file_checked(actual_rendered_tex, wgpu::TextureFormat::Rgba8UnormSrgb, &png_path).expect("Save failed");

    });
}
