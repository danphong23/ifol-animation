# Báo Cáo Kiểm Thử: TC02 - Bộ Nhớ Thành Phần & Quản Lý Hủy (Component Storage & Drop Safety)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice02_component.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice02_component.rs)  
> **Module liên quan:** [`src/storage/sparse_set.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/storage/sparse_set.rs), [`src/storage/any_storage.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/storage/any_storage.rs), [`src/world/world.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/world/world.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC02` (Slice 02)
- **Tên:** Lưu trữ Component, theo dõi phiên bản cấu trúc (Structural Version), Drop Safety và bảo toàn mảng đặc khi xóa hàng loạt
- **Mục tiêu kiểm thử:**
  1. Kiểm tra `structural_version` tăng đơn điệu khi `spawn`, `insert` kiểu component mới hoặc `remove`.
  2. Đảm bảo việc chỉnh sửa dữ liệu thông thường (`get_mut`) KHÔNG làm tăng `structural_version` mà chỉ cập nhật `dense_ticks`.
  3. Kiểm tra destructor (`Drop`) của component được gọi chính xác 1 lần duy nhất khi entity bị despawn (`DropTracker`).
  4. Tạo 100 entities có component `Position`, xóa entity ở vị trí 50 (`middle_e`), kiểm tra thuật toán `swap_remove` bảo toàn dữ liệu của 99 entities còn lại.

---

## 2. Sơ Đồ Trực Quan Quá Trình Tính Toán & Chuyển Đổi Trạng Thái

```text
┌────────────────────────────────────────────────────────────────────────┐
│ World::new()                                                           │
│ └── structural_version = 0, current_tick = 0                           │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ spawn(e1)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ structural_version = 1                                                 │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ insert Position(10, 20) & Velocity(1, 2)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ structural_version = 3 (Tăng vì thay đổi Topology kiểu dữ liệu)       │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ get_mut::<Position>(e1) -> pos.x += 5.0
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ structural_version GIỮ NGUYÊN = 3 | get_tick::<Position>(e1) = Some(1) │
│ (Chỉnh sửa dữ liệu không làm vô hiệu hóa bộ đệm truy vấn)              │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Gắn DropTracker vào e2 -> despawn(e2)
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 🛡️ DROP SAFETY: drop_counter: 0 -> 1 (Gọi chính xác 1 lần duy nhất)   │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Tạo 100 entities -> Xóa entity thứ 50
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ 📦 SWAP_REMOVE: Dồn phần tử 99 vào vị trí 50 trong mảng đặc (O(1))     │
│    └── 99 entities còn lại giữ nguyên 100% dữ liệu chính xác           │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Tiêu chí kiểm tra | Kỳ vọng (Expected) | Thực tế (Actual) | Kết luận |
| :--- | :--- | :--- | :---: |
| **Tăng Structural Version** | Tăng khi thay đổi topology | `0 -> 1 -> 2 -> 3` | **ĐẠT** |
| **Không tăng khi mutate data** | `get_mut` giữ nguyên version | `structural_version == 3` | **ĐẠT** |
| **Cập nhật Change Tick** | `get_mut` cập nhật tick | `get_tick(e1) == Some(1)` | **ĐẠT** |
| **An toàn thu hồi bộ nhớ (Drop)** | Gọi đúng 1 lần khi despawn | `drop_counter == 1` | **ĐẠT** |
| **Toàn vẹn 99 Entities sau xóa** | 99 phần tử còn lại giữ nguyên tọa độ | `get(e) == Some(Position(i, i))` | **ĐẠT** |
| **Entity bị xóa trả về None** | `has_component(e50) == false` | `get(e50) == None` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất
- **Thời gian thực thi:** `~2 µs`
- **Bộ nhớ:** Các component được nén chặt trong mảng đặc `dense_data: Vec<T>`, xóa bỏ hoàn toàn overhead phân mảnh bộ nhớ (Memory Fragmentation).
- **Trạng thái:** **ĐẠT (PASS ✅)**
