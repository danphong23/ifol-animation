mod support;

use ifol_ecs::error::SystemError;
use ifol_ecs::schedule::PhaseId;
use ifol_ecs::system::AccessDescriptor;
use ifol_ecs::EcsRuntime;
use std::fs;
use std::path::Path;
use support::{FailingSystem, Health};

#[test]
fn slice07_system_context_isolation_and_structured_errors() {
    let mut runtime = EcsRuntime::new();

    runtime.register_component::<Health>().unwrap();
    let phase = PhaseId::Update;
    runtime.register_phase(phase.clone()).unwrap();

    // 1. Register a system that mutates health via SystemContext
    let sys_ok = runtime
        .register_function_system(
            "HealSystem",
            |ctx| {
                let e_candidates: Vec<ifol_ecs::EntityId> = ctx
                    .query::<&'static Health>()
                    .iter_with_entity()
                    .map(|(e, _)| e)
                    .collect();

                for e in e_candidates {
                    if let Some(h) = ctx.get_mut::<Health>(e) {
                        h.0 += 50;
                    }
                }
                Ok(())
            },
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    // 2. Register a system that intentionally returns SystemError
    let sys_fail = runtime
        .register_system(
            "FailingSystem",
            FailingSystem,
            AccessDescriptor::new(),
            vec![],
        )
        .unwrap();

    runtime.attach_system(&phase, sys_ok).unwrap();
    runtime.attach_system(&phase, sys_fail).unwrap();
    runtime.compile().unwrap();

    // Spawn an entity with Health 50
    let e = runtime.spawn();
    runtime.insert(e, Health(50)).unwrap();

    // 3. Execute pass
    let report = runtime.run_once().unwrap();

    // Verify HealSystem ran and updated health
    assert_eq!(report.systems_executed, vec!["HealSystem"]);
    assert_eq!(runtime.get::<Health>(e), Some(&Health(100)));

    // Verify FailingSystem produced structured diagnostic error
    assert_eq!(report.system_errors.len(), 1);
    assert_eq!(report.system_errors[0].0, "FailingSystem");
    assert_eq!(
        report.system_errors[0].1,
        SystemError::new("intentional test failure")
    );

    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 07 - System Context & Structured Diagnostics

> **Tài liệu đối chiếu:** `docs/05_system_model.md`, `docs/10_contracts_and_diagnostics.md`

---

## 1. Kết Quả Kiểm Thử

| Tiêu Chí Kiểm Tra | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: |
| **Bảo vệ ranh giới qua `SystemContext`** | Truy cập an toàn, không rò rỉ `&mut World` | **PASS** |
| **Thực thi logic thành công (`HealSystem`)** | Máu tăng từ 50 lên 100 chính xác | **PASS** |
| **Ghi nhận `SystemError` có cấu trúc** | Bắt lỗi `intentional test failure` | **PASS** |
| **Không Panic làm crash runtime** | Runtime thu thập lỗi vào `RunReport` an toàn | **PASS (Fail-Safe)** |
"#;
    fs::write(reports_dir.join("slice07_system_report.md"), report_content).unwrap();
}
