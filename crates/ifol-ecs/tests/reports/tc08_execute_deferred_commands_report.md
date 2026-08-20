# Báo Cáo Kiểm Thử: TC08 - Thực Thi Lập Lịch, Điểm An Toàn & Flush Commands (Schedule & Safe Points)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice08_execute.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice08_execute.rs)  
> **Module liên quan:** [`src/schedule/compiled.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/schedule/compiled.rs), [`src/system/commands.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/commands.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC08` (Slice 08)
- **Tên:** Thực thi Schedule đa Phase, điểm an toàn (Safe Points) và áp dụng lệnh trì hoãn (`Commands`) giữa các Phase
- **Mục tiêu kiểm thử:**
  1. Tạo 2 Phase: `prepare` và `observe` (`prepare` chạy trước `observe`).
  2. `DeferredSpawnerSystem` trong Phase 1 tìm kiếm Entity có `Position` và đẩy lệnh hoãn `insert(e, Health(200))` vào `ctx.commands()`.
  3. Kiểm tra cơ chế Flush tại **Safe Point**: Lệnh được áp dụng vào `World` trước khi Phase 2 bắt đầu.
  4. `HealthReaderSystem` trong Phase 2 đọc `Health` và xác nhận giá trị `Health(200)` đã có sẵn mà không bị lỗi trễ một frame (*Zero Frame Delay*).

---

## 2. Sơ Đồ Trực Quan Quá Trình Thực Thi & Flush Tại Safe Point

```text
┌───────────────────────────────┐
│       PHASE 1: prepare        │
│   (DeferredSpawnerSystem)     │
│   ctx.commands().insert(...)  │
└──────────────┬────────────────┘
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│ 🛡️ PHASE BOUNDARY SAFE POINT                                │
│   • commands.apply(&mut world)                              │
│   • Áp dụng Health(200) vào Entity e                        │
│   • commands_processed = 1                                  │
└──────────────┬──────────────────────────────────────────────┘
               │
               ▼
┌───────────────────────────────┐
│       PHASE 2: observe        │
│    (HealthReaderSystem)       │
│   Đọc được ngay Health(200)!  │
└───────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Tiêu chí kiểm tra | Kỳ vọng | Thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **Số Phase đã duyệt** | 2 Phase (`prepare`, `observe`) | `report.phases_visited.len() == 2` | **ĐẠT** |
| **Số System đã chạy** | 2 System (`DeferredSpawnerSystem`, `HealthReaderSystem`) | `report.systems_executed.len() == 2` | **ĐẠT** |
| **Số lệnh Commands đã Flush**| $\ge 1$ lệnh tại Safe Point | `report.commands_processed == 1` | **ĐẠT** |
| **Dữ liệu đọc ở Phase 2** | Nhìn thấy ngay `Health(200)` | `healths == vec![200]` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất
- **Thời gian thực thi:** `~37 µs`
- **Đánh giá:** Các lệnh biến đổi cấu trúc được gom lại và áp dụng tức thì tại ranh giới Phase, vừa đảm bảo an toàn con trỏ, vừa không gây ra độ trễ 1 frame giữa các Phase trong cùng một Pass.
- **Trạng thái:** **ĐẠT (PASS ✅)**
