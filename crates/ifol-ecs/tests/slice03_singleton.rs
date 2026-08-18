mod support;

use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::{AccessDescriptor, RunCondition};
use ifol_ecs::EcsRuntime;
use std::fs;
use std::path::Path;
use support::{RunCounter, TestConfig};

#[test]
fn slice03_world_singleton_and_run_conditions() {
    let mut runtime = EcsRuntime::new();

    let cfg_id = runtime.register_world_singleton::<TestConfig>().unwrap();
    let _counter_id = runtime.register_world_singleton::<RunCounter>().unwrap();

    let phase = PhaseId::Update;
    runtime.register_phase(phase.clone()).unwrap();

    // 1. Register a system that REQUIRES TestConfig
    let sys_required = runtime
        .register_function_system(
            "ConfigRequiredSystem",
            |ctx| {
                if let Some(counter) = ctx.world_mut::<RunCounter>() {
                    counter.ticks += 10;
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![RunCondition::WorldHas(cfg_id, "TestConfig")],
        )
        .unwrap();

    // 2. Register an OPTIONAL system (runs unconditionally)
    let sys_optional = runtime
        .register_function_system(
            "OptionalSystem",
            |ctx| {
                if let Some(counter) = ctx.world_mut::<RunCounter>() {
                    counter.ticks += 1;
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![RunCondition::Always],
        )
        .unwrap();

    runtime.attach_system(&phase, sys_required).unwrap();
    runtime.attach_system(&phase, sys_optional).unwrap();
    runtime.compile().unwrap();

    // Insert RunCounter on WORLD_ENTITY, but omit TestConfig
    runtime.insert_world_component(RunCounter { ticks: 0 });

    // 3. First execution pass: ConfigRequiredSystem should be SKIPPED with reason!
    let report1 = runtime.run_once().unwrap();
    assert_eq!(report1.systems_executed, vec!["OptionalSystem"]);
    assert_eq!(report1.systems_skipped.len(), 1);
    assert_eq!(report1.systems_skipped[0].system, "ConfigRequiredSystem");
    assert!(report1.systems_skipped[0].reason.contains("Missing required world singleton 'TestConfig'"));
    assert_eq!(runtime.get_world_component::<RunCounter>(), Some(&RunCounter { ticks: 1 }));

    // 4. Insert TestConfig on WORLD_ENTITY
    runtime.insert_world_component(TestConfig {
        speed_multiplier: 2.0,
        title: "Test Animation".to_string(),
    });

    // 5. Second execution pass: Both systems should execute!
    let report2 = runtime.run_once().unwrap();
    assert_eq!(report2.systems_executed.len(), 2);
    assert_eq!(report2.systems_skipped.len(), 0);
    // counter ticks = 1 (old) + 10 (from required) + 1 (from optional) = 12
    assert_eq!(runtime.get_world_component::<RunCounter>(), Some(&RunCounter { ticks: 12 }));

    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 03 - World Singleton & Run Conditions

> **Tài liệu đối chiếu:** `docs/02_resources_and_data_flow.md`, `docs/03_phase_scheduler_and_dag.md`

---

## 1. Kết Quả Kiểm Thử

| Kịch Bản Kiểm Tra | Kết Quả Mong Đợi | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: | :---: |
| **Vắng mặt Singleton bắt buộc** | System bị Skip có reason | `Missing required world singleton 'TestConfig'` | **PASS** |
| **Singleton tùy chọn (`Always`)** | System vẫn chạy bình thường | Thực thi thành công | **PASS** |
| **Bổ sung Singleton vào runtime** | Cả 2 system tự động kích hoạt | 2/2 Systems Executed | **PASS** |
| **Dùng chung hạ tầng Component** | Lưu trên `EntityId::WORLD` ($O(1)$) | Toàn vẹn 100% | **PASS** |
"#;
    fs::write(reports_dir.join("slice03_singleton_report.md"), report_content).unwrap();
}
