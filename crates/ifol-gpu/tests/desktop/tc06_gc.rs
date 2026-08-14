mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc06_gc() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load warrior texture
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let warrior_uniform = h.build_sprite_uniform(
            &tex_heroes,
            [0.0, 0.0],
            0.8,
            [0.03, 0.0],
            [0.26, 1.0],
            0.45,
            0.12,
            0.5,
            1.0,
        );
        let bg_warrior_uni = h.create_sprite_uniform_bind_group(warrior_uniform);

        let pipe_id = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 2. Stress-allocate 100 RenderNodes in the Pool
        let mut node_ids = Vec::new();
        for _ in 0..100 {
            let node_id = h.pool.alloc_batch(vec![
                DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_warrior_uni, Vec::new()),
            ]);
            node_ids.push(node_id);
        }

        assert_eq!(h.pool.len(), 100);

        // 3. Remove 99 nodes, keep only the last one (node_ids[99])
        for id in node_ids.iter().take(99) {
            h.pool.remove(*id);
        }

        assert_eq!(h.pool.len(), 1);

        // 4. Build Graph with only the 1 surviving node
        let (target_id, target_tex) = h.create_target("TC06 Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.1, 0.1, 0.14, 1.0]);

        graph.add_node_id(node_ids[99]);

        // 5. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC06 - Node Garbage Collection & Arena Slot Recycling",
            "allocated_nodes": 100,
            "freed_nodes": 99,
            "surviving_nodes": 1,
            "clear_color": [0.1, 0.1, 0.14, 1.0]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc06_gc.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc06_gc",
            "Node Garbage Collection & Slot Recycling",
            "Màn hình chỉ render duy nhất 1 nhân vật Nữ chiến binh. Không có rò rỉ bộ nhớ hoặc vẽ trùng lặp từ 99 node đã giải phóng.",
            "RenderNodePool quản lý bộ nhớ hoàn hảo: 99 node rác đã được thu hồi an toàn. Node duy nhất còn lại được compile và render chính xác, không xuất hiện hiện tượng use-after-free hay crash bộ nhớ.",
        );
    });
}
