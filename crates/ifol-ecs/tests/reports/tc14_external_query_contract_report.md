# Báo Cáo Kiểm Thử: TC14 - Hợp Đồng Truy Vấn Công Khai Ngoại Tuyến (External Query Contract)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice14_external_query.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice14_external_query.rs)  
> **Module liên quan:** [`src/query/query_item.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/query_item.rs), [`src/world/world.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/world/world.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC14` (Slice 14)
- **Tên:** Kiểm tra khả năng triển khai `WorldQuery` tự định nghĩa từ bên ngoài (*Custom External Query Term*) và công bố hợp đồng `QueryAccess`
- **Mục tiêu kiểm thử:**
  1. Tạo kiểu truy vấn tự định nghĩa `struct PositionPresence` thực thi `WorldQuery`.
  2. Khai báo `has_driver() = true` trỏ tới `world.component_entities::<Position>()`.
  3. Truy vấn `world.query::<PositionPresence>()` trên thực thể có `Position` $\rightarrow$ Khớp thành công 1 entity.
  4. Hợp nhất `QueryAccess::read::<Position>()` với `QueryAccess::write::<Velocity>()` $\rightarrow$ Hợp lệ (`validate_mutable().is_ok()`).
  5. Tạo xung đột alias: Vừa đọc vừa ghi `Position` $\rightarrow$ Từ chối (`validate_mutable().is_err()`).

---

## 2. Sơ Đồ Trực Quan Tích Hợp Trait WorldQuery Ngoại Tuyến

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. ĐỊNH NGHĨA CUSTOM QUERY TERM TỪ BÊN NGOÀI               │
│    struct PositionPresence;                                 │
│    impl WorldQuery for PositionPresence { ... }             │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. CÔNG BỐ HỢP ĐỒNG TRUY CẬP (ACCESS CONTRACT)             │
│    • access(): QueryAccess::read::<Position>()              │
│    • driver_entities(): world.component_entities::<Pos>()   │
│    • matches(): world.has_component::<Position>(entity)     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 3. TRUY VẤN: world.query::<PositionPresence>()              │
│    └── Tương thích 100% với Query Engine nội bộ của ECS     │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Tiêu chí | Kỳ vọng | Thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **Duyệt Custom WorldQuery** | Đếm đúng số Entity có Position | `world.query::<PositionPresence>().count() == 1` | **ĐẠT** |
| **Driver Selection công khai** | Sử dụng `component_entities::<T>` | Trả về danh sách Entity liên tục | **ĐẠT** |
| **Hợp nhất QueryAccess** | Merge read(Pos) + write(Vel) | `validate_mutable().is_ok() == true` | **ĐẠT** |
| **Phát hiện Alias nội bộ** | Thêm write(Pos) vào read(Pos) | `validate_mutable().is_err() == true` | **ĐẠT** |

---

## 4. Phân Tích & Tính Tương Thích
- **Thời gian thực thi:** `~18 µs`
- **Đánh giá:** Giao diện `WorldQuery` được thiết kế công khai và mở rộng tự do, cho phép các crate tầng trên (`ifol-engine`, `feature-render-core`) viết các kiểu lọc chuyên dụng riêng mà vẫn đảm bảo an toàn truy cập.
- **Trạng thái:** **ĐẠT (PASS ✅)**
