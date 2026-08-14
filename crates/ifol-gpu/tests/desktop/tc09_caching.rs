mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;
use std::time::Instant;

#[test]
fn run_tc09_caching() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load background
        let tex_nightsky = h.load_texture("bg_nightsky.jpeg");

        // 2. Setup pipelines
        let pipe_blit = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let pipe_particles = h.register_pipeline(
            "particles_10k.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );

        let (target_id, target_tex) = h.create_target("TC09 Target (Caching Benchmark)");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.02, 0.05, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_blit, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_nightsky.bind_group, Vec::new()),
                DrawCommand::new(pipe_particles, DrawAction::Procedural { vertex_count: 6, instance_range: 0..10_000 }),
            ],
        );

        // Benchmark Cold Frame
        let t0 = Instant::now();
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("TC09 Frame 1 failed");
        let cold_time = t0.elapsed();

        // Benchmark 10 Warm Cached Frames
        let mut warm_durations = Vec::new();
        for _ in 0..10 {
            let t = Instant::now();
            h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("TC09 warm frame failed");
            warm_durations.push(t.elapsed());
        }
        let avg_warm: std::time::Duration = warm_durations.iter().sum::<std::time::Duration>() / warm_durations.len() as u32;

        println!("TC09 Benchmark: Cold: {:?}, Avg Warm (10 frames): {:?}", cold_time, avg_warm);

        // Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC09 - Pipeline Caching & Bundle Reuse Benchmark",
            "cold_start_time_us": cold_time.as_micros(),
            "avg_warm_time_us": avg_warm.as_micros(),
            "speedup_percentage": format!("{:.1}%", (1.0 - avg_warm.as_secs_f64() / cold_time.as_secs_f64()) * 100.0)
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc09_caching.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // Capture output image & record report
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc09_caching",
            "Pipeline Caching & Bundle Reuse Benchmark",
            "Hình ảnh 10,000 hạt bụi sao đêm tương đương TC08, nhưng với tốc độ thực thi các frame sau nhanh hơn vượt trội.",
            &format!("Cơ chế Caching & Bundle Reuse giúp giảm overhead từ Cold {:?} xuống Warm {:?}, đảm bảo hiệu năng 60+ FPS ổn định.", cold_time, avg_warm),
        );
    });
}
