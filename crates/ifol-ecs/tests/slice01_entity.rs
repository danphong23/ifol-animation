mod support;

use ifol_ecs::entity::{EntityId, EntityManager};
use ifol_ecs::error::EcsError;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[test]
fn slice01_entity_lifecycle_and_generational_safety() {
    let mut mgr = EntityManager::new();

    // 1. Initial state
    assert_eq!(mgr.alive_count(), 1); // WORLD entity
    assert!(mgr.is_alive(EntityId::WORLD));

    // 2. Spawn 100 entities
    let mut entities = Vec::with_capacity(100);
    for _ in 0..100 {
        entities.push(mgr.spawn());
    }
    assert_eq!(mgr.alive_count(), 101);

    // 3. Despawn even index entities
    let mut despawned_indices = HashSet::new();
    for (idx, &e) in entities.iter().enumerate() {
        if idx % 2 == 0 {
            assert!(mgr.despawn(e).is_ok());
            despawned_indices.insert(e.index());
        }
    }
    assert_eq!(mgr.alive_count(), 51);

    // 4. Stale ID rejection
    for (idx, &e) in entities.iter().enumerate() {
        if idx % 2 == 0 {
            assert!(!mgr.is_alive(e));
            assert_eq!(mgr.despawn(e), Err(EcsError::EntityNotFound(e)));
        } else {
            assert!(mgr.is_alive(e));
        }
    }

    // 5. Forged ID rejection on free slots
    for &free_idx in &despawned_indices {
        let forged_next_gen = EntityId::new(free_idx, 2);
        assert!(!mgr.is_alive(forged_next_gen));
        assert_eq!(
            mgr.validate(forged_next_gen),
            Err(EcsError::ForgedEntityId(forged_next_gen))
        );
    }

    // 6. Recycled slot allocation increments generation
    let mut new_entities = Vec::new();
    for _ in 0..50 {
        new_entities.push(mgr.spawn());
    }
    assert_eq!(mgr.alive_count(), 101);

    for &new_e in &new_entities {
        assert!(despawned_indices.contains(&new_e.index()));
        assert_eq!(new_e.generation(), 2);
        assert!(mgr.is_alive(new_e));
    }

    // 7. Protection of WORLD entity
    assert_eq!(
        mgr.despawn(EntityId::WORLD),
        Err(EcsError::EntityNotFound(EntityId::WORLD))
    );

    // Write visual test report
    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 01 - Entity Lifecycle & Generational Safety

> **Tài liệu đối chiếu:** `docs/01_world_storage_and_query.md`, `docs/11_test_and_acceptance_map.md`

---

## 1. Kết Quả Kiểm Thử

| Tiêu Chí Kiểm Tra | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: |
| **Bảo vệ `WORLD_ENTITY` (Slot 0, Gen 1)** | Không thể despawn/recycle | **PASS** |
| **Cấp phát & Thu hồi Slot** | Tái sử dụng chính xác 50 slot tự do | **PASS** |
| **Tăng thế hệ (Generation Increment)** | Tăng từ `gen 1` lên `gen 2` | **PASS** |
| **Từ chối Handle cũ (Stale ID Rejection)** | Báo lỗi `EntityNotFound` | **PASS** |
| **Từ chối Handle giả mạo (Forged ID Rejection)** | Báo lỗi `ForgedEntityId` | **PASS** |
"#;
    fs::write(reports_dir.join("slice01_entity_report.md"), report_content).unwrap();
}
