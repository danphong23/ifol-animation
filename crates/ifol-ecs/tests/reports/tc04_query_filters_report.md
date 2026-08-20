# Báo Cáo Kiểm Thử: TC04 - Công Cụ Truy Vấn, Bộ Lọc & Driver Selection (Query & Filters)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice04_query.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice04_query.rs)  
> **Module liên quan:** [`src/query/query.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/query.rs), [`src/query/query_item.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/query_item.rs), [`src/query/filter.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/query/filter.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC04` (Slice 04)
- **Tên:** Truy vấn Tuple phức tạp, lựa chọn Driver thu hẹp nhất (Most Restrictive Driver) và các bộ lọc `With`, `Without`, `Option`
- **Mục tiêu kiểm thử:**
  1. `Query<&Position>` bao gồm cả 10 entities thông thường và 1 `EntityId::WORLD` (tổng = 11).
  2. `Query<(&Position, &Velocity)>` tự động loại bỏ `WORLD_ENTITY` do không có `Velocity` (khớp đúng 10 entities).
  3. Lọc khẳng định: `With<OptionalTag>` lọc đúng 5 entities có tag chẵn.
  4. Lọc phủ định: `Without<OptionalTag>` lọc đúng 6 entities (5 entity lẻ + 1 WORLD_ENTITY).
  5. Lọc mở rộng: `Option<&OptionalTag>` trả về đầy đủ 11 entities (`Some` cho entity chẵn, `None` cho entity lẻ và WORLD).
  6. Truy vấn rộng 8 thành phần (`WideQuery`): Kết hợp đồng thời 8 điều kiện khác nhau và duyệt chính xác.

---

## 2. Sơ Đồ Trực Quan Cơ Chế Lựa Chọn Driver & Lọc Candidate

```text
┌─────────────────────────────────────────────────────────────┐
│ TRUY VẤN: Query<(&Position, With<OptionalTag>)>             │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 1. DRIVER SELECTION: So sánh kích thước tập hợp             │
│    • Component Position    : 11 entities                    │
│    • Component OptionalTag :  5 entities                    │
│    └── Chọn OptionalTag làm DRIVER (Tập nhỏ nhất = 5) ⚡    │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ 2. FILTER EVALUATION: Lọc trên 5 candidates của driver      │
│    • Entity 2 : Có Position? ✅ -> YIELD                     │
│    • Entity 4 : Có Position? ✅ -> YIELD                     │
│    • Entity 6 : Có Position? ✅ -> YIELD                     │
│    • Entity 8 : Có Position? ✅ -> YIELD                     │
│    • Entity 10: Có Position? ✅ -> YIELD                     │
│    └── KẾT QUẢ: Khớp chính xác 5 / 5 entities                │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Ma Trận Dữ Liệu Thực Thể & Kết Quả Truy Vấn

| Thực thể | Position | Velocity | Health | OptionalTag | Name | Khớp &Position | Khớp (&Pos, &Vel) | Khớp With&lt;Tag&gt; | Khớp Without&lt;Tag&gt; | Khớp WideQuery (8 Terms) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`WORLD_ENTITY`** | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ (1/11) | ❌ | ❌ | ✅ (1/6) | ❌ |
| **`e1 (Lẻ)`** | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ (2/11) | ✅ (1/10) | ❌ | ✅ (2/6) | ✅ (1/10) |
| **`e2 (Chẵn)`**| ✅ | ✅ | ✅ | ✅ | ❌ | ✅ (3/11) | ✅ (2/10) | ✅ (1/5) | ❌ | ✅ (2/10) |
| **`e3 (Lẻ)`** | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ (4/11) | ✅ (3/10) | ❌ | ✅ (3/6) | ✅ (3/10) |
| **`e4 (Chẵn)`**| ✅ | ✅ | ✅ | ✅ | ❌ | ✅ (5/11) | ✅ (4/10) | ✅ (2/5) | ❌ | ✅ (4/10) |
| **`e5..e10`** | ✅ | ✅ | ✅ | 3 chẵn, 3 lẻ | ❌ | ✅ (11/11) | ✅ (10/10) | ✅ (5/5) | ✅ (6/6) | ✅ (10/10) |

---

## 4. Kết Quả & Đánh Giá Chi Tiết

| Chữ ký Query | Số lượng kỳ vọng | Số lượng thực tế | Đánh giá |
| :--- | :---: | :---: | :---: |
| `Query<&Position>` | 11 | 11 | **ĐẠT** |
| `Query<(&Position, &Velocity)>` | 10 | 10 | **ĐẠT** |
| `Query<(&Position, With<OptionalTag>)>` | 5 | 5 | **ĐẠT** |
| `Query<(&Position, Without<OptionalTag>)>` | 6 | 6 | **ĐẠT** |
| `Query<(&Position, Option<&OptionalTag>)>` | 11 | 11 | **ĐẠT** |
| `Query<&Name>` (Không entity nào có) | 0 (`is_empty = true`) | 0 | **ĐẠT** |
| `Query<WideQuery>` (8 thành phần) | 10 | 10 | **ĐẠT** |

---

## 5. Phân Tích Hiệu Suất
- **Thời gian thực thi:** `~95 µs`
- **Thuật toán Driver Selection:** Query Engine so sánh `len()` của `Position`, `Velocity`, `Health`, `OptionalTag` và chọn driver có tập candidate nhỏ nhất (`OptionalTag` với 5 phần tử khi có `With<OptionalTag>`), giảm số vòng lặp từ 11 xuống 5.
- **Trạng thái:** **ĐẠT (PASS ✅)**
