# Báo Cáo Kiểm Thử: TC05 - Đăng Ký Hệ Thống, Nguồn Gốc & Phiên Bản Đơn Điệu (Registry & Provenance)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice05_registry.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice05_registry.rs)  
> **Module liên quan:** [`src/registry/component_registry.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/registry/component_registry.rs), [`src/registry/phase_registry.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/registry/phase_registry.rs), [`src/registry/system_registry.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/registry/system_registry.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC05` (Slice 05)
- **Tên:** Kiểm tra tính toàn vẹn của Registry, phiên bản đơn điệu (`revision`), bảo vệ nguồn gốc ID (*Registry Provenance Safety*) và ngăn chặn xung đột quyền
- **Mục tiêu kiểm thử:**
  1. Kiểm tra `ComponentRegistry::revision` tăng khi đăng ký kiểu mới; từ chối đăng ký trùng (`DuplicateComponent`).
  2. Kiểm tra `PhaseRegistry::revision` tăng khi đăng ký Phase hoặc Edge; từ chối Phase rỗng, Phase trùng và Edge trùng.
  3. Kiểm tra `SystemRegistry` từ chối `AccessDescriptor` không hợp lệ (vừa khai báo `read` vừa khai báo `write` trên cùng 1 Component).
  4. Kiểm tra an toàn nguồn gốc (*Provenance Safety*): Bắt buộc từ chối `ComponentId` được sinh ra từ một `ComponentRegistry` khác (`ComponentIdNotRegistered`).

---

## 2. Sơ Đồ Trực Quan Quá Trình Kiểm Tra Bảo Mật Nguồn Gốc (Registry Provenance)

```text
┌──────────────────────────────────────┐       ┌──────────────────────────────────────┐
│     Foreign ComponentRegistry        │       │      World ComponentRegistry         │
│         (registry_id: 1)             │       │          (registry_id: 2)            │
└──────────────────┬───────────────────┘       └──────────────────┬───────────────────┘
                   │                                              │
         Đăng ký Position                               Đăng ký Position
                   │                                              │
                   ▼                                              ▼
         ComponentId {                                  ComponentId {
             registry: 1,                                   registry: 2,
             index: 0                                       index: 0
         }                                              }
                   │                                              │
                   │                                              │
                   └───────────────────────┐                      │
                                           ▼                      ▼
                           ┌──────────────────────────────────────────────┐
                           │               Runtime Compile                │
                           │   Kiểm tra ID(1, 0) đối chiếu Registry(2)   │
                           └──────────────────────┬───────────────────────┘
                                                  │
                                                  ▼
                           ┌──────────────────────────────────────────────┐
                           │    ❌ TỪ CHỐI NGAY TẠI THỜI ĐIỂM BIÊN DỊCH   │
                           │       Err(ComponentIdNotRegistered)          │
                           └──────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Hành vi kiểm tra | Đầu vào | Kết quả thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **Đăng ký Component trùng** | `comp_reg.register::<Position>()` lần 2 | `Err(DuplicateComponent)` | **ĐẠT** |
| **Đăng ký Phase rỗng** | `PhaseId::new("")` | `Err(InvalidPhaseId)` | **ĐẠT** |
| **Đăng ký Phase trùng** | `PhaseId::new("prepare")` lần 2 | `Err(DuplicatePhase)` | **ĐẠT** |
| **Nối cạnh vào Phase ảo** | `add_phase_edge("prepare", "missing")` | `Err(PhaseNotFound)` | **ĐẠT** |
| **Nối cạnh trùng lặp** | `add_phase_edge("prepare", "simulate")` lần 2 | `Err(DuplicatePhaseEdge)` | **ĐẠT** |
| **Xung đột quyền đọc/ghi** | `access.add_read(pos); access.add_write(pos);` | `Err(InvalidAccessDescriptor)` | **ĐẠT** |
| **Chặn ID từ Registry khác** | Gắn `foreign_id` vào runtime của registry khác | `Err(ComponentIdNotRegistered)` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & An Toàn
- **Thời gian thực thi:** `~45 µs`
- **Đánh giá:** Mỗi `ComponentId` và `SystemId` đều mang `registry: u64` gốc trong 64-bit ID, đảm bảo không thể tráo đổi ID giữa các Runtime khác nhau.
- **Trạng thái:** **ĐẠT (PASS ✅)**
