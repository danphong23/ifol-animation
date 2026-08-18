mod support;

use ifol_ecs::error::EcsError;
use ifol_ecs::registry::{ComponentRegistry, PhaseId, PhaseRegistry, SystemRegistry};
use ifol_ecs::system::{AccessDescriptor, FunctionSystem};
use std::fs;
use std::path::Path;
use support::{Position, Velocity};

#[test]
fn slice05_registry_transactional_commit_and_revisions() {
    // 1. ComponentRegistry
    let mut comp_reg = ComponentRegistry::new();
    let id_pos = comp_reg.register::<Position>().unwrap();
    let id_vel = comp_reg.register::<Velocity>().unwrap();
    assert_eq!(comp_reg.revision(), 2);

    // Duplicate component registration
    assert_eq!(
        comp_reg.register::<Position>(),
        Err(EcsError::DuplicateComponent(std::any::type_name::<Position>()))
    );

    // 2. PhaseRegistry
    let mut phase_reg = PhaseRegistry::new();
    phase_reg.register_phase(PhaseId::PreUpdate).unwrap();
    phase_reg.register_phase(PhaseId::Update).unwrap();
    assert_eq!(phase_reg.revision(), 2);

    // Duplicate phase registration
    assert_eq!(
        phase_reg.register_phase(PhaseId::PreUpdate),
        Err(EcsError::DuplicatePhase("PreUpdate".to_string()))
    );

    // Unknown phase dependency edge
    assert_eq!(
        phase_reg.add_phase_edge(&PhaseId::PreUpdate, &PhaseId::custom("NonExistent")),
        Err(EcsError::PhaseNotFound("NonExistent".to_string()))
    );

    // 3. SystemRegistry & AccessDescriptor validation
    let mut sys_reg = SystemRegistry::new();

    // Invalid access descriptor: reading and writing the same component
    let mut invalid_access = AccessDescriptor::new();
    invalid_access.add_read(id_pos);
    invalid_access.add_write(id_pos);

    let fail_result = sys_reg.register(
        "InvalidSystem".to_string(),
        Box::new(FunctionSystem::new(|_| Ok(()))),
        invalid_access,
        vec![],
    );
    assert!(matches!(fail_result, Err(EcsError::InvalidAccessDescriptor(_, _))));

    // Valid access descriptor
    let mut valid_access = AccessDescriptor::new();
    valid_access.add_read(id_pos);
    valid_access.add_write(id_vel);

    let sys_id = sys_reg
        .register(
            "ValidSystem".to_string(),
            Box::new(FunctionSystem::new(|_| Ok(()))),
            valid_access,
            vec![],
        )
        .unwrap();

    assert_eq!(sys_reg.revision(), 1);
    assert!(sys_reg.get(sys_id).is_some());

    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 05 - Registries & Access Contracts

> **Tài liệu đối chiếu:** `docs/03_registry_and_api.md`, `docs/05_system_model.md`

---

## 1. Kết Quả Kiểm Thử

| Tiêu Chí Kiểm Tra | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: |
| **Trùng lặp Component ID** | Báo lỗi `DuplicateComponent` | **PASS** |
| **Trùng lặp Phase ID** | Báo lỗi `DuplicatePhase` | **PASS** |
| **Nối cạnh vào Phase không tồn tại** | Báo lỗi `PhaseNotFound` | **PASS** |
| **Mâu thuẫn Read/Write Access** | Báo lỗi `InvalidAccessDescriptor` | **PASS (Chống Aliasing)** |
| **Theo dõi Monotonic Revision** | Tăng chính xác trên mỗi mutation | **PASS** |
"#;
    fs::write(reports_dir.join("slice05_registry_report.md"), report_content).unwrap();
}
