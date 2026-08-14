mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc07_recursion() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load assets
        let tex_scifi_bg = h.load_texture("bg_scifi.jpeg");
        let tex_props = h.load_texture("bg_forest_props1.jpeg");
        let tex_monsters = h.load_texture("sprites_monsters.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_items = h.load_texture("sprites_items.jpeg");

        // 2. Setup pipelines
        let pipe_blit = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        let (target_id, target_tex) = h.create_target("TC07 Output Target");

        // 3. Level 5 (Deepest - E): Draws Background & clears canvas
        let mut graph_e = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        let node_e = h.pool.alloc_batch(vec![
            DrawCommand::new(pipe_blit, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_scifi_bg.bind_group, Vec::new()),
        ]);
        graph_e.add_node_id(node_e);

        // 4. Level 4 (D): Embeds E + Draws Tree Prop
        let tree_uni = h.build_sprite_uniform(
            &tex_props,
            [-0.45, -0.1],
            0.85,
            [0.0, 0.0],
            [0.18, 0.42],
            0.40,
            0.10,
            0.5,
            1.0,
        );
        let bg_tree_uni = h.create_sprite_uniform_bind_group(tree_uni);

        let mut graph_d = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        });
        let node_d = h.pool.alloc_subgraph(
            "SubGraph E (Background)",
            graph_e,
            vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_props.bind_group, Vec::new())
                .with_bind_group(1, bg_tree_uni, Vec::new())],
        );
        graph_d.add_node_id(node_d);

        // 5. Level 3 (C): Embeds D + Draws Golem Monster
        let golem_uni = h.build_sprite_uniform(
            &tex_monsters,
            [-0.05, -0.15],
            0.7,
            [0.68, 0.5],
            [0.98, 0.98],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_golem_uni = h.create_sprite_uniform_bind_group(golem_uni);

        let mut graph_c = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        });
        let node_c = h.pool.alloc_subgraph(
            "SubGraph D (Tree)",
            graph_d,
            vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_monsters.bind_group, Vec::new())
                .with_bind_group(1, bg_golem_uni, Vec::new())],
        );
        graph_c.add_node_id(node_c);

        // 6. Level 2 (B): Embeds C + Draws Wizard Hero
        let wizard_uni = h.build_sprite_uniform(
            &tex_heroes,
            [0.35, -0.1],
            0.75,
            [0.30, 0.0],
            [0.52, 1.0],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_wizard_uni = h.create_sprite_uniform_bind_group(wizard_uni);

        let mut graph_b = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        });
        let node_b = h.pool.alloc_subgraph(
            "SubGraph C (Golem)",
            graph_c,
            vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                .with_bind_group(1, bg_wizard_uni, Vec::new())],
        );
        graph_b.add_node_id(node_b);

        // 7. Level 1 (Root - A): Embeds B + Draws Items (Chest)
        let chest_uni = h.build_sprite_uniform(
            &tex_items,
            [0.0, -0.4],
            0.4,
            [0.0, 0.5],
            [0.5, 1.0],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_chest_uni = h.create_sprite_uniform_bind_group(chest_uni);

        let mut graph_a = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        });

        let node_a = h.pool.alloc_subgraph(
            "SubGraph B (Wizard)",
            graph_b,
            vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, tex_items.bind_group, Vec::new())
                .with_bind_group(1, bg_chest_uni, Vec::new())],
        );
        graph_a.add_node_id(node_a);

        // 8. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC07 - Deep Recursion SubGraphs (5 Levels)",
            "hierarchy": "Root (Chest) -> Sub B (Wizard) -> Sub C (Golem) -> Sub D (Tree) -> Sub E (Background)",
            "depth": 5,
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc07_recursion.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 9. Execute & Record
        h.execute_and_record(
            &graph_a,
            &target_tex,
            "tc07_recursion",
            "Deep Recursion SubGraphs (5 Levels Deep)",
            "Đồ thị đệ quy 5 cấp lồng nhau (SciFi BG + Cây sồi + Golem + Pháp sư + Rương báu) được duỗi phẳng và hiển thị trọn vẹn cả 5 lớp.",
            "Trình biên dịch Topological Graph Compiler duỗi phẳng thành công 5 cấp đồ thị đệ quy mà không gây tràn stack. Tất cả 5 lớp hình ảnh hiển thị đúng thứ tự không gian và hòa trộn sắc nét.",
        );
    });
}
