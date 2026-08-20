# Báo Cáo Kiểm Thử: TC12 - Truy Vấn Đột Biến, Chống Aliasing & An Toàn Mượn (Mutable Query & Anti-Aliasing)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice12_mutable_query.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice12_mutable_query.rs)  
> **Module liên quan:** [`src/query/query_mut.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/query_mut.rs), [`src/query/query_item.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/query_item.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC12` (Slice 12)
- **Tên:** Kiểm tra an toàn truy vấn đột biến `WorldQueryMut`, cơ chế chống Aliasing (trùng lặp tham chiếu mutable) và cập nhật tick
- **Mục tiêu kiểm thử:**
  1. Kiểm tra `query_mut::<(&mut Position, &Velocity)>()` vừa đột biến `Position` vừa đọc `Velocity` an toàn, cập nhật đúng `current_tick`.
  2. Kiểm tra đột biến kèm `Option<&mut OptionalTag>`: Chỉ tăng tọa độ cho các entity có tag (`x += 10.0`).
  3. **Chống Aliasing (Anti-Aliasing):** Từ chối ngay lập tức các chữ ký Query có nguy cơ mượn hai con trỏ mutable tới cùng 1 kiểu component:
     - `(&mut Position, &mut Position)` $\rightarrow$ **REJECT**.
     - `(&Position, &mut Position)` $\rightarrow$ **REJECT**.
  4. Kiểm tra `SystemContext::query_mut` bắt buộc đối chiếu với `AccessDescriptor` trước khi cấp quyền lặp.

---

## 2. Sơ Đồ Trực Quan Cơ Chế Chống Aliasing của QueryAccess

```text
┌─────────────────────────────────────────────────────────────┐
│                 Yêu cầu: world.query_mut::<Q>()             │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│              Phân tích QueryAccess::<Q>()                   │
│   • writes: Tập hợp TypeId các component yêu cầu ghi        │
│   • reads : Tập hợp TypeId các component yêu cầu đọc        │
└──────────────────────────────┬──────────────────────────────┘
                               │
            ┌──────────────────┴──────────────────┐
            ▼                                     ▼
┌──────────────────────────────┐    ┌──────────────────────────────┐
│  writes có phần tử trùng?    │    │  writes giao với reads?      │
│  ví dụ: (&mut Pos, &mut Pos) │    │  ví dụ: (&Pos, &mut Pos)     │
└──────────────┬───────────────┘    └──────────────┬───────────────┘
               │ CÓ                                │ CÓ
               └──────────────────┬────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────┐
│ ❌ TỪ CHỐI NGAY LẬP TỨC: Err(AliasedAccess)                 │
│    "mutable query contains aliased component access"        │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Chữ ký Query / Thao tác | Hành vi | Kết quả thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| `query_mut::<(&mut Pos, &Vel)>()` | Cập nhật $x = x + v_x, y = y + v_y$ | `e1.pos = (4.0, 6.0)`, `e2.pos = (9.0, 22.0)` | **ĐẠT** |
| `query_mut::<(&mut Pos, Option<&mut Tag>)>()` | Chỉ cộng thêm 10 cho entity có Tag | `e2.pos.x: 9.0 -> 19.0` (tagged = 1) | **ĐẠT** |
| `query_mut::<(&mut Pos, &mut Pos)>()` | Cố tình mượn 2 mutable alias | `Err(AliasedAccess)` | **ĐẠT** |
| `query_mut::<(&Pos, &mut Pos)>()` | Vừa mượn read vừa mượn write alias | `Err(AliasedAccess)` | **ĐẠT** |
| `SystemContext::query_mut` | Khớp quyền `write(Position)` | `entity.pos.x = 1.0 + 5.0 = 6.0` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & An Toàn Con Trỏ
- **Thời gian thực thi:** `~35 µs`
- **Đánh giá:** Cơ chế xác thực `QueryAccess::validate_mutable()` đảm bảo triệt để quy tắc Borrowing của Rust (Chỉ 1 mutable XOR nhiều immutable), triệt tiêu hoàn toàn nguy cơ Undefined Behavior khi duyệt mảng.
- **Trạng thái:** **ĐẠT (PASS ✅)**
