mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc08_massive() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load night sky background
        let tex_nightsky = h.load_texture("bg_nightsky.jpeg");

        // 2. Setup pipelines
        let pipe_blit = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let pipe_particles = h.register_pipeline(
            "particles_10k.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );

        // 3. Create target
        let (target_id, target_tex) = h.create_target("TC08 Target (10k Particles)");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.02, 0.05, 1.0]);

        // 4. Add Blit Background + 10,000 Particle Instances
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_blit, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_nightsky.bind_group, Vec::new()),
                DrawCommand::new(pipe_particles, DrawAction::Procedural { vertex_count: 6, instance_range: 0..10_000 }),
            ],
        );

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC08 - Massive Draw Commands VS Massive Instances (10,000 particles)",
            "instances": 10000,
            "background": "bg_nightsky.jpeg",
            "shader": "particles_10k.wgsl"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc08_massive.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc08_massive",
            "Massive Draw Commands (10,000 Instanced Dust Particles)",
            "Bầu trời đêm anime huyền ảo với 10,000 hạt bụi sao phát sáng (vàng, lục, trắng) phân bố giả ngẫu nhiên khắp không gian.",
            "Engine xử lý 10,000 instance đồ họa một cách mượt mà và tức thì. Không có độ trễ hay nghẽn cổ chai bộ đệm GPU.",
        );
    });
}
