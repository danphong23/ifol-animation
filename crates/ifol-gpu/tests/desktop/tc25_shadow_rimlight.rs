mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RimUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    rim_color: [f32; 3],
    rim_thickness: f32,
    shadow_offset: [f32; 2],
    shadow_color: [f32; 4],
    _pad: [f32; 2],
}

#[test]
fn run_tc25_shadow_rimlight() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_rim = h.register_pipeline(
            "rimlight.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let s_y = 1.5f32;
        let s_x = s_y * (600.0 / 800.0) * (0.275 / 0.97);

        let uniform = RimUniform {
            transform: [
                [s_x, 0.0, 0.0, 0.0],
                [0.0, s_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            rim_color: [1.0, 1.0, 0.0], // Yellow Rim light
            rim_thickness: 8.0,         // 8 pixels
            shadow_offset: [0.05, -0.05], // Drop shadow offset
            shadow_color: [0.0, 0.0, 0.0, 0.6], // Semi-transparent black
            _pad: [0.0; 2],
        };

        let bg_rim = h.create_custom_uniform_bind_group(uniform, "Rim Uniform");

        let (target_id, target_tex) = h.create_target("TC25 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                // 2 instances: 0 = Shadow pass, 1 = Main pass with Rim
                DrawCommand::new(pipe_rim, DrawAction::Procedural { vertex_count: 6, instance_range: 0..2 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_rim, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC25 - Fake Rim Lighting & Drop Shadow",
            "features": [
                "2-pass instancing (1 shadow, 1 main)",
                "Edge detection rim lighting in shader",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc25_shadow_rimlight.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc25_shadow_rimlight",
            "Fake Rim Lighting & Drop Shadow",
            "Dùng Instancing để vẽ 2 pass trong 1 draw call: Pass đầu (index 0) là Drop Shadow đổ bóng đen. Pass thứ hai (index 1) là nhân vật chính kèm hiệu ứng Edge Detection viền sáng mờ (Rim Light) xung quanh nhân vật.",
            "Tạo hiệu ứng nổi 2.5D cho Sprite phẳng, giúp nhân vật không bị chìm vào phông nền phía sau mà không cần tạo model 3D.",
        );
    });
}
