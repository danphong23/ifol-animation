mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc03_zbuffer() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load textures
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_monsters = h.load_texture("sprites_monsters.jpeg");
        let tex_forest = h.load_texture("bg_forest_props1.jpeg");

        // 2. Setup 3 Sprites with aspect ratio correction
        // Warrior (Z = 0.8, Furthest back, placed at X = -0.25)
        let warrior_uniform = h.build_sprite_uniform(
            &tex_heroes,
            [-0.25, -0.1],
            0.75,
            [0.03, 0.0],
            [0.26, 1.0],
            0.45,
            0.12,
            0.8,
            1.0,
        );
        let bg_warrior_uni = h.create_sprite_uniform_bind_group(warrior_uniform);

        // Tree (Z = 0.2, Closest in front, placed at X = 0.0)
        let tree_uniform = h.build_sprite_uniform(
            &tex_forest,
            [0.0, 0.0],
            0.85,
            [0.0, 0.0],
            [0.18, 0.42],
            0.40,
            0.10,
            0.2,
            1.0,
        );
        let bg_tree_uni = h.create_sprite_uniform_bind_group(tree_uniform);

        // Golem (Z = 0.5, Middle depth, placed at X = 0.25)
        let golem_uniform = h.build_sprite_uniform(
            &tex_monsters,
            [0.25, -0.1],
            0.75,
            [0.68, 0.5],
            [0.98, 0.98],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_golem_uni = h.create_sprite_uniform_bind_group(golem_uniform);

        // 3. Register Pipeline with Depth Test enabled
        let pipe_id = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::REPLACE),
            true, // Enable Depth Test
            true,
        );

        // 4. Create Target & Depth Attachment
        let (target_id, target_tex) = h.create_target("TC03 Color Target");
        let (depth_id, _depth_tex) = h.create_depth_target("TC03 Depth Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.12, 0.15, 1.0])
        .with_depth_stencil(depth_id);

        // Submit in deliberate order: Warrior (0.8) -> Tree (0.2) -> Golem (0.5)
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_warrior_uni, Vec::new()),
                DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree_uni, Vec::new()),
                DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_monsters.bind_group, Vec::new())
                    .with_bind_group(1, bg_golem_uni, Vec::new()),
            ],
        );

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC03 - Z-Buffer Culling & Depth Testing",
            "clear_color": [0.1, 0.12, 0.15, 1.0],
            "depth_stencil": "Depth32Float",
            "layers": [
                { "name": "Tree", "z_depth": 0.2, "status": "In Front" },
                { "name": "Golem", "z_depth": 0.5, "status": "Middle" },
                { "name": "Warrior", "z_depth": 0.8, "status": "Behind" }
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc03_zbuffer.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc03_zbuffer",
            "Z-Buffer Culling & Depth Testing",
            "Cây (Z=0.2) che khuất một phần Golem (Z=0.5) và Nữ chiến binh (Z=0.8). Thứ tự lớp hoàn toàn chính xác theo Z-Buffer.",
            "Các vật thể lồng lên nhau chính xác theo chiều sâu Z: Cây sồi (Z=0.2) nằm trên cùng che Golem (Z=0.5) và Nữ chiến binh (Z=0.8). Không có hiện tượng Z-fighting hay sai lệch thứ tự vẽ.",
        );
    });
}
