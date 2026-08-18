mod support;

use ifol_ecs::World;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use support::{DropTracker, Position, Velocity};

#[test]
fn slice02_component_storage_drop_and_revisions() {
    let mut world = World::new();

    // 1. Initial structural version
    assert_eq!(world.structural_version(), 0);

    let e1 = world.spawn();
    assert_eq!(world.structural_version(), 1);

    // 2. Insert components increments structural version
    world.insert(e1, Position { x: 10.0, y: 20.0 }).unwrap();
    assert_eq!(world.structural_version(), 2);

    world.insert(e1, Velocity { dx: 1.0, dy: 2.0 }).unwrap();
    assert_eq!(world.structural_version(), 3);

    // 3. Modifying component via get_mut increments change tick but DOES NOT increment structural version
    let struct_ver_before_mut = world.structural_version();
    if let Some(pos) = world.get_mut::<Position>(e1) {
        pos.x += 5.0;
    }
    assert_eq!(world.structural_version(), struct_ver_before_mut);
    assert_eq!(world.get_tick::<Position>(e1), Some(1));

    // 4. Drop tracking: Verify DropTracker drop is called exactly once on despawn
    let drop_counter = Arc::new(AtomicUsize::new(0));
    let e2 = world.spawn();
    world
        .insert(
            e2,
            DropTracker {
                counter: Arc::clone(&drop_counter),
            },
        )
        .unwrap();

    assert_eq!(drop_counter.load(Ordering::SeqCst), 0);

    // Despawn e2
    assert!(world.despawn(e2).is_ok());
    assert_eq!(drop_counter.load(Ordering::SeqCst), 1);

    // 5. Swap-remove integrity on bulk entities
    let mut bulk_entities = Vec::new();
    for i in 0..100 {
        let e = world.spawn();
        world.insert(e, Position { x: i as f32, y: i as f32 }).unwrap();
        bulk_entities.push(e);
    }

    // Remove middle entity
    let middle_e = bulk_entities[50];
    world.remove::<Position>(middle_e);
    assert!(!world.has_component::<Position>(middle_e));

    // Verify other entities remain 100% accessible
    for (i, &e) in bulk_entities.iter().enumerate() {
        if i == 50 {
            assert_eq!(world.get::<Position>(e), None);
        } else {
            assert_eq!(
                world.get::<Position>(e),
                Some(&Position { x: i as f32, y: i as f32 })
            );
        }
    }

    let reports_dir = Path::new("tests/reports");
    fs::create_dir_all(reports_dir).unwrap();
    let report_content = r#"# Báo Cáo Chấp Nhận: Slice 02 - Component Storage, Drop & Revisions

> **Tài liệu đối chiếu:** `docs/01_world_storage_and_query.md`, `docs/07_cache_and_revision.md`

---

## 1. Kết Quả Kiểm Thử

| Tiêu Chí Kiểm Tra | Kết Quả Thực Tế | Đánh Giá |
| :--- | :---: | :---: |
| **Phân tách `structural_version` vs Data Mutation** | Sửa giá trị không làm tăng structural version | **PASS** |
| **Bảo đảm Drop Lifecycle (`DropTracker`)** | Drop được gọi chính xác 1 lần khi despawn | **PASS (Zero Leaks)** |
| **Bảo toàn tính liên tục khi `swap_remove`** | 99 entity còn lại nguyên vẹn 100% | **PASS** |
| **Change Tick trên từng thực thể** | Đánh dấu chính xác tick sửa đổi | **PASS** |
"#;
    fs::write(reports_dir.join("slice02_component_report.md"), report_content).unwrap();
}
