use std::time::Instant;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};

mod harness;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BoneUniform {
    bones: [[[f32; 4]; 4]; 4],
}

fn make_transform(tx: f32, ty: f32, rot_deg: f32, sx: f32, sy: f32) -> [[f32; 4]; 4] {
    let rad = rot_deg.to_radians();
    let c = rad.cos();
    let s = rad.sin();
    
    [
        [c * sx, s * sx, 0.0, 0.0],
        [-s * sy, c * sy, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [tx, ty, 0.0, 1.0],
    ]
}

fn mat_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut res = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            res[r][c] = a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c] + a[r][3] * b[3][c];
        }
    }
    res
}

#[test]
fn test_tc84_skeletal_anim() {
    pollster::block_on(async {
        let mut h = harness::DesktopTestHarness::new(800, 600).await;
        let (target_handle, target_tex) = h.create_target("tc84_target");

        // Bone Hierarchy:
        // Torso (Root) -> Head
        // Torso -> Arm
        // Torso -> Leg
        let torso_local = make_transform(0.0, -0.1, 0.0, 1.2, 1.5);
        let head_local = make_transform(0.0, 0.35, 15.0, 0.8, 0.8);
        let arm_local = make_transform(0.25, 0.1, -45.0, 0.6, 1.2);
        let leg_local = make_transform(-0.15, -0.35, 20.0, 0.6, 1.3);

        let torso_world = torso_local;
        let head_world = mat_mul(&torso_world, &head_local);
        let arm_world = mat_mul(&torso_world, &arm_local);
        let leg_world = mat_mul(&torso_world, &leg_local);

        let uniform_data = BoneUniform {
            bones: [torso_world, head_world, arm_world, leg_world],
        };

        let (_, uniform_buf) = h.create_storage_buffer(&[uniform_data], "bone_uniform", wgpu::BufferUsages::UNIFORM);

        let bgl = h.engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skeletal_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });

        let bg = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skeletal_bg"), layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &uniform_buf, offset: 0, size: None }) },
            ],
        });
        let bg_h = h.insert_bind_group(bg, 1);

        let shader_code = std::fs::read_to_string("tests/shared_assets/shaders/render_skeletal.wgsl").unwrap();
        let shader = h.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("render_skeletal.wgsl"), source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(&shader_code)),
        });

        let layout = h.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skeletal_layout"), bind_group_layouts: &[Some(&bgl)], immediate_size: 0,
        });

        let pipe = h.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_skeletal"), layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader, entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        });
        let pipe_h = h.insert_pipeline(pipe, vec![Some(1)]);

        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target_handle, width: 800, height: 600 });
        
        let draw_cmd = DrawCommand::new(
            pipe_h,
            DrawAction::Procedural { vertex_count: 24, instance_range: 0..1 },
        ).with_bind_group(0, bg_h, Vec::new());
        
        graph.add_batch(&mut h.pool, vec![draw_cmd]);

        let t_start = Instant::now();
        let sub = h.executor.execute(&h.engine, &h.registry, &mut h.pool, &graph).unwrap();
        let _ = h.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(sub), timeout: None });
        let t_elapsed = t_start.elapsed();
        println!("2D Skeletal Animation Render Time: {:?}", t_elapsed);

        h.execute_and_record(&graph, &target_tex, "tc84_skeletal_anim", "2D Skeletal Hierarchy Animation", "Evaluating 2D bone matrix hierarchy transformations for body parts", "Render output");
    });
}
