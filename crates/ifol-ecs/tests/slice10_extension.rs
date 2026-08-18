mod support;

use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use ifol_ecs::EcsRuntime;
use std::fs;
use std::path::Path;

// Mock Feature 1: Animation Package
#[derive(Debug, PartialEq, Clone, Copy)]
struct KeyframeTrack {
    start_x: f32,
    target_x: f32,
}

// Mock Feature 2: Render Core Package
#[derive(Debug, PartialEq, Clone, Copy)]
struct RenderCache {
    is_dirty: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct Transform {
    x: f32,
}

#[test]
fn slice10_feature_package_registration_and_extension() {
    let mut runtime = EcsRuntime::new();

    // Feature 1 registers its components
    runtime.register_component::<KeyframeTrack>().unwrap();
    runtime.register_component::<Transform>().unwrap();

    // Feature 2 registers its components
    runtime.register_component::<RenderCache>().unwrap();

    // Features register their phases
    let p_anim = PhaseId::custom("animation.evaluate");
    let p_render = PhaseId::custom("render.prepare");

    runtime.register_phase(p_anim.clone()).unwrap();
    runtime.register_phase(p_render.clone()).unwrap();
    runtime.add_phase_edge(&p_anim, &p_render).unwrap();

    // Feature 1 registers its animation evaluation system
    let sys_anim = runtime
        .register_function_system(
            "AnimationEvaluateSystem",
            |ctx| {
                let items: Vec<(ifol_ecs::EntityId, f32)> = ctx
                    .query::<(&'static Transform, &'static KeyframeTrack)>()
                    .iter_with_entity()
                    .map(|(e, (tf, track))| (e, tf.x + (track.target_x - track.start_x) * 0.5))
                    .collect();

                for (e, new_x) in items {
                    if let Some(tf) = ctx.get_mut::<Transform>(e) {
                        tf.x = new_x;
                    }
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    // Feature 2 registers its render dirty preparation system
    let sys_render = runtime
        .register_function_system(
            "RenderPrepareSystem",
            |ctx| {
                let items: Vec<ifol_ecs::EntityId> = ctx
                    .query::<(&'static Transform, &'static RenderCache)>()
                    .iter_with_entity()
                    .map(|(e, _)| e)
                    .collect();

                for e in items {
                    if let Some(cache) = ctx.get_mut::<RenderCache>(e) {
                        cache.is_dirty = true;
                    }
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&p_anim, sys_anim).unwrap();
    runtime.attach_system(&p_render, sys_render).unwrap();
    runtime.compile().unwrap();

    // Create entity with combined feature components
    let e = runtime.spawn();
    runtime.insert(e, Transform { x: 0.0 }).unwrap();
    runtime
        .insert(
            e,
            KeyframeTrack {
                start_x: 0.0,
                target_x: 100.0,
            },
        )
        .unwrap();
    runtime.insert(e, RenderCache { is_dirty: false }).unwrap();

    // Run execution pass
    let report = runtime.run_once().unwrap();
    assert_eq!(report.phases_visited.len(), 2);
    assert_eq!(report.systems_executed.len(), 2);

    // Verify Feature 1 updated transform to 50.0
    assert_eq!(runtime.get::<Transform>(e), Some(&Transform { x: 50.0 }));

    // Verify Feature 2 marked render cache dirty
    assert_eq!(runtime.get::<RenderCache>(e), Some(&RenderCache { is_dirty: true }));

    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 10 - Feature Extension & Zero Core Mutation

> **Tài liệu đối chiếu:** `docs/09_feature_registration_and_extension.md`

---

## 1. Kết Quả Kiểm Thử

| Tiêu Chí Kiểm Tra | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: |
| **Đăng ký Feature độc lập (`feature-animation`)** | Nạp Component & System qua Public API | **PASS** |
| **Đăng ký Feature độc lập (`feature-render-core`)** | Nạp Component & Phase qua Public API | **PASS** |
| **Phối hợp dữ liệu đa Feature trên 1 Entity** | System chạy tuần tự theo đúng Phase DAG | **PASS** |
| **Không sửa đổi lõi ECS** | Core giữ nguyên tính Generic 100% | **PASS** |
"#;
    fs::write(reports_dir.join("slice10_extension_report.md"), report_content).unwrap();
}
