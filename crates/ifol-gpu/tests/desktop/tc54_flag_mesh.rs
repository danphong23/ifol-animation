mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::{
    MeshHandle, MeshResourceDescriptor, PipelineHandle, PipelineLayoutResourceDescriptor,
};
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FlagUniform {
    time: f32,
    wave_freq: f32,
    wave_amp: f32,
    _pad: f32,
}

#[test]
fn run_tc54_flag_mesh() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_scifi = h.load_texture("bg_scifi.jpeg");

        // 1. Generate 32x32 Procedural Mesh Grid (1089 vertices, 2048 triangles, 6144 indices)
        let grid_size: u32 = 32;
        let mut vertices = Vec::new();
        for y in 0..=grid_size {
            for x in 0..=grid_size {
                let u = x as f32 / grid_size as f32;
                let v = y as f32 / grid_size as f32;
                // Position in NDC [-0.7, 0.7] x [-0.5, 0.5]
                let px = u * 1.4 - 0.7;
                let py = (1.0 - v) * 1.0 - 0.5;
                vertices.push(Vertex {
                    pos: [px, py, 0.0],
                    uv: [u, v],
                });
            }
        }

        let mut indices: Vec<u16> = Vec::new();
        for y in 0..grid_size {
            for x in 0..grid_size {
                let i0 = (y * (grid_size + 1) + x) as u16;
                let i1 = (y * (grid_size + 1) + (x + 1)) as u16;
                let i2 = ((y + 1) * (grid_size + 1) + x) as u16;
                let i3 = ((y + 1) * (grid_size + 1) + (x + 1)) as u16;

                // Triangle 1: i0, i2, i1
                indices.push(i0);
                indices.push(i2);
                indices.push(i1);

                // Triangle 2: i1, i2, i3
                indices.push(i1);
                indices.push(i2);
                indices.push(i3);
            }
        }

        let vb = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flag Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let ib = h.engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Flag Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        });

        let mesh_id = MeshHandle(54);
        let num_indices = indices.len() as u32;
        let num_vertices = vertices.len() as u32;
        let vb_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        let ib_size = (indices.len() * std::mem::size_of::<u16>()) as u64;

        h.registry.insert_mesh_with_descriptor(
            mesh_id,
            (vb, Some((ib, wgpu::IndexFormat::Uint16)), num_indices),
            MeshResourceDescriptor {
                vertex_buffer_size: vb_size,
                vertex_count: num_vertices,
                index_buffer_size: Some(ib_size),
                index_format: Some(wgpu::IndexFormat::Uint16),
            },
        ).unwrap();

        // 2. Create custom pipeline with VertexBufferLayout
        let shader_code = fs::read_to_string(Path::new("tests/shared_assets/shaders/flag_mesh.wgsl")).unwrap();
        let shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flag_mesh.wgsl"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
        });

        let pipeline_layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("flag_pipeline_layout"),
            bind_group_layouts: &[
                Some(&h.texture_bg_layout),
                Some(&h.uniform_bg_layout),
            ],
            immediate_size: 0,
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };

        let pipeline = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Flag Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(vertex_layout)],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: None, // Double sided flag
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let pipe_id = PipelineHandle(54);
        h.registry.insert_pipeline_with_layout_descriptor(
            pipe_id,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(1), Some(2)],
            },
        );

        let flag_uni = FlagUniform {
            time: 1.2,
            wave_freq: 6.0,
            wave_amp: 0.15,
            _pad: 0.0,
        };
        let bg_flag_uni = h.create_custom_uniform_bind_group(flag_uni, "Flag Uniform");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.08, 0.08, 0.12, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_id, DrawAction::Indexed {
                    mesh: mesh_id,
                    index_range: 0..num_indices,
                    instance_range: 0..1,
                })
                .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                .with_bind_group(1, bg_flag_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC54 - High-Density Procedural 3D Flag Mesh",
            "features": [
                "32x32 Grid Mesh (1089 Vertices, 6144 Indices)",
                "Vertex & Index Buffer Hardware Binding",
                "Vertex Shader 3D Wave Displacement & Normal Lighting"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc54_flag_mesh.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &final_target_tex,
            "tc54_flag_mesh",
            "3D Flag Mesh Wave Displacement",
            "Lưới đa giác mật độ cao 32x32 (1,089 đỉnh, 6,144 chỉ số) được uốn lượn 3D trong Vertex Shader mô phỏng lá cờ bay trong gió với hiệu ứng chiếu sáng Phong Lighting.",
            "Xác thực toàn bộ luồng tạo, đăng ký và thực thi Vertex Buffer / Index Buffer thực tế kết hợp DrawAction::Indexed.",
        );
    });
}
