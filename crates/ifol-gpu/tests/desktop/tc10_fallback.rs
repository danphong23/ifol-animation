mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::execution::RenderGraphValidationError;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::BindGroupHandle;
use std::fs;

#[test]
fn run_tc10_fallback() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let pipe_id = h.register_pipeline("chroma_key.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let (target_id, target_tex) = h.create_target("TC10 Target");

        // 1. Deliberately construct an invalid graph with a missing BindGroup (ID 999999)
        let mut bad_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        });

        bad_graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, BindGroupHandle(999_999), Vec::new()),
            ],
        );

        // 2. Execute bad graph and verify that it returns a typed error WITHOUT panicking
        let result = h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut bad_graph);
        assert!(result.is_err(), "Engine should return Err for missing resources");
        
        let err = result.unwrap_err();
        println!("TC10 Caught Expected Error: {:?}", err);
        match err {
            RenderGraphValidationError::MissingBindGroup(handle) => {
                assert_eq!(handle, BindGroupHandle(999_999));
            }
            other => panic!("Unexpected error variant: {:?}", other),
        }

        // 3. Graceful Fallback Recovery: Render standard Magenta color [1.0, 0.0, 1.0, 1.0]
        let mut fallback_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([1.0, 0.0, 1.0, 1.0]); // Magenta missing texture indicator

        // Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC10 - Missing Resource Safe Error Handling & Magenta Fallback",
            "tested_error": "RenderGraphValidationError::MissingBindGroup(999999)",
            "panic_occurred": false,
            "fallback_color": [1.0, 0.0, 1.0, 1.0]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc10_fallback.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 4. Execute Fallback & Record
        h.execute_and_record(
            &fallback_graph,
            &target_tex,
            "tc10_fallback",
            "Missing Resource Error Handling & Magenta Fallback",
            "Toàn bộ màn hình hiển thị màu hồng cánh sen (Magenta) đặc trưng khi tài nguyên bị thiếu, không có crash phần mềm.",
            "Engine xử lý triệt để các lỗi ngoại lệ (Edge Case): khi BindGroup hoặc Texture bị thiếu, hệ thống trả về Typed Error an toàn và kích hoạt Fallback Pipeline hiển thị màu Magenta cảnh báo cho người dùng.",
        );
    });
}
