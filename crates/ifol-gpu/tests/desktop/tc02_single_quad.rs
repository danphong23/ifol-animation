mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc02_single_quad() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load texture from shared assets
        let tex_info = h.load_texture("sprites_heroes.jpeg");

        // 2. Crop Wizard (X: 30%..52%, Y: 0%..100%) with aspect ratio correction
        let wizard_uniform = h.build_sprite_uniform(
            &tex_info,
            [0.0, 0.0],
            0.8, // Target height scale
            [0.30, 0.0],
            [0.52, 1.0],
            0.45, // Tolerance
            0.12, // Smoothness
            0.5,  // Z-depth
            1.0,  // Opacity
        );
        let uniform_bg_id = h.create_sprite_uniform_bind_group(wizard_uniform);

        // 3. Register Pipeline with Chroma Key and Alpha Blending
        let pipe_id = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 4. Create Target & Build Graph
        let (target_id, target_tex) = h.create_target("TC02 Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.08, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipe_id,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, tex_info.bind_group, Vec::new())
                .with_bind_group(1, uniform_bg_id, Vec::new()),
            ],
        );

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC02 - Single Quad Sprite with Chroma Key",
            "clear_color": [0.05, 0.05, 0.08, 1.0],
            "nodes": [
                {
                    "pipeline": "chroma_key_cropped.wgsl",
                    "sprite": "Wizard (sprites_heroes.jpeg)",
                    "crop_uv": [0.25, 0.0, 0.50, 1.0],
                    "position": [0.0, 0.0]
                }
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc02_single_quad.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc02_single_quad",
            "Single Quad Sprite with Chroma Key",
            "1 nhân vật Pháp sư tóc xanh đứng giữa màn hình, phông nền xanh đã được lọc sạch hoàn toàn trên nền tối.",
            "Nhân vật Pháp sư đứng giữa màn hình sắc nét. Viền phông xanh lục đã bị loại bỏ triệt để bởi shader Chroma Key. Không có artifact hay viền xanh thừa.",
        );
    });
}
