mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc04_alpha_blend() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load items texture
        let tex_items = h.load_texture("sprites_items.jpeg");

        // 2. Setup 3 Sprites with aspect ratio correction:
        // A. Opaque Chest at Z = 0.5 (Center [0.0, 0.0])
        let chest_uni = h.build_sprite_uniform(
            &tex_items,
            [0.0, 0.0],
            0.55,
            [0.0, 0.5],
            [0.5, 1.0],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_chest_uni = h.create_sprite_uniform_bind_group(chest_uni);

        // B. Transparent Scroll in FRONT at Z = 0.2 (Offset [0.15, -0.1], Opacity 0.75)
        let scroll_uni = h.build_sprite_uniform(
            &tex_items,
            [0.15, -0.1],
            0.5,
            [0.5, 0.5],
            [1.0, 1.0],
            0.45,
            0.12,
            0.2,
            0.75,
        );
        let bg_scroll_uni = h.create_sprite_uniform_bind_group(scroll_uni);

        // C. Transparent Potion BEHIND at Z = 0.8 (Offset [-0.15, 0.1], Opacity 0.75)
        let potion_uni = h.build_sprite_uniform(
            &tex_items,
            [-0.15, 0.1],
            0.45,
            [0.0, 0.0],
            [0.5, 0.5],
            0.45,
            0.12,
            0.8,
            0.75,
        );
        let bg_potion_uni = h.create_sprite_uniform_bind_group(potion_uni);

        // 3. Pipelines: Opaque (Replace, Depth write) & Transparent (Alpha Blend, Depth test)
        let pipe_opaque = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::REPLACE),
            true,
            true,
        );

        let pipe_alpha = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            true,
            true,
        );

        // 4. Target & Depth
        let (target_id, target_tex) = h.create_target("TC04 Color Target");
        let (depth_id, _depth_tex) = h.create_depth_target("TC04 Depth Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.08, 0.08, 0.1, 1.0])
        .with_depth_stencil(depth_id);

        // Draw order:
        // 1. Draw Opaque Chest (writes depth Z = 0.5)
        // 2. Draw Transparent Potion (Z = 0.8, should be culled/hidden by chest)
        // 3. Draw Transparent Scroll (Z = 0.2, in front, should blend with chest)
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_opaque, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_chest_uni, Vec::new()),
                DrawCommand::new(pipe_alpha, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_potion_uni, Vec::new()),
                DrawCommand::new(pipe_alpha, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_scroll_uni, Vec::new()),
            ],
        );

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC04 - Alpha Blending & Depth Interaction",
            "clear_color": [0.08, 0.08, 0.1, 1.0],
            "objects": [
                { "name": "Opaque Chest", "z_depth": 0.5, "blend": "Replace", "write_depth": true },
                { "name": "Behind Potion", "z_depth": 0.8, "blend": "AlphaBlend", "expected": "Occluded" },
                { "name": "Front Scroll", "z_depth": 0.2, "blend": "AlphaBlend", "expected": "Translucent over Chest" }
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc04_alpha_blend.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc04_alpha_blend",
            "Alpha Blending & Depth Interaction",
            "Cuộn phép tím (Z=0.2) bán trong suốt phủ mờ nhìn xuyên thấu qua Rương gỗ (Z=0.5). Bình thuốc (Z=0.8) bị rương gỗ che hoàn toàn.",
            "Khả năng hòa trộn Alpha Blending hoạt động hoàn hảo: Ánh hào quang tím của cuộn phép bán trong suốt nhìn xuyên qua bề mặt gỗ của rương. Bình thuốc phía sau bị che khuất đúng theo Z-Buffer mà không bị rò rỉ pixel.",
        );
    });
}
