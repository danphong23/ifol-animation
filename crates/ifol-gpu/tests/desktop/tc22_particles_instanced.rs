mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InstancedUniform {
    aspect_ratio: f32,
    time: f32,
    _pad: [f32; 2],
}

#[test]
fn run_tc22_particles_instanced() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        // Use the heroes sprite sheet
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_instanced = h.register_pipeline(
            "instanced_prop.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true, // use sprite layout (t_diffuse, s_diffuse)
        );

        let uniform = InstancedUniform {
            aspect_ratio: 800.0 / 600.0,
            time: 0.0,
            _pad: [0.0; 2],
        };

        let bg_instanced = h.create_custom_uniform_bind_group(uniform, "Instanced Uniform");

        let (target_id, target_tex) = h.create_target("TC22 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.15, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_instanced, DrawAction::Procedural { vertex_count: 6, instance_range: 0..100 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_instanced, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC22 - Hardware Instancing",
            "features": [
                "100 instances of a prop",
                "Procedural instance transform generation",
                "Aspect ratio correction per instance",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc22_particles_instanced.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc22_particles_instanced",
            "Hardware Instancing (Props)",
            "Render 100 instance của 1 vật phẩm (Prop) bằng cách dùng chung 1 lệnh draw.",
            "Test khả năng tối ưu draw call của ECS khi có nhiều hạt hoặc prop giống nhau.",
        );
    });
}
