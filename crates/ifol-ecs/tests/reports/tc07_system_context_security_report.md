# Báo Cáo Kiểm Thử: TC07 - Sandbox SystemContext, Cách Ly & Xử Lý Lỗi Có Cấu Trúc (System Security)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice07_system.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice07_system.rs)  
> **Module liên quan:** [`src/system/context.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/context.rs), [`src/system/access.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/access.rs), [`src/report.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/report.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC07` (Slice 07)
- **Tên:** Kiểm tra Sandbox `SystemContext`, thu giữ lỗi có cấu trúc (`SystemError`), phạm vi `SystemId` theo Runtime và điều hướng lỗi bằng `ExecutionPolicy`
- **Mục tiêu kiểm thử:**
  1. Cho `HealSystem` cập nhật `Health` từ 50 lên 100 thành công thông qua `SystemContext`.
  2. Cho `FailingSystem` cố tình trả về `Err(SystemError)` $\rightarrow$ Kiểm tra runtime không panic, thu giữ lỗi vào `report.system_errors`.
  3. Kiểm tra `SystemId` từ một Runtime ngoại lai không thể gán vào Runtime hiện tại.
  4. Kiểm tra hành vi truy cập component không khai báo trong `AccessDescriptor` $\rightarrow$ Trả về `SystemError::access_denied`.
  5. Kiểm tra 3 chính sách xử lý lỗi `ExecutionPolicy`:
     - `CollectErrors`: Ghi lỗi và chạy tiếp.
     - `StopPhaseOnError`: Dừng Phase hiện tại, bỏ qua các System sau trong Phase đó.
     - `FailFast`: Hủy bỏ ngay lập tức pass và trả về `Err(SystemExecutionFailed)`.

---

## 2. Sơ Đồ Trực Quan Luồng Điều Phối Lỗi & Sandbox Access Check

```text
┌─────────────────────────────────────────────────────────────┐
│                 SystemContext Execution Gate                │
│             Hệ thống gọi: ctx.get_mut::<Health>(e)          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│ Kiểm tra AccessDescriptor: Có quyền write(Health) không?    │
└──────────────┬───────────────────────────────┬──────────────┘
               │ CÓ                            │ KHÔNG
               ▼                               ▼
┌─────────────────────────────┐ ┌─────────────────────────────┐
│ ✅ HỢP LỆ: CẬP NHẬT HEALTH  │ │ ❌ BỊ CHẶN: ACCESS DENIED   │
│   Health: 50 -> 100         │ │   Trả về SystemError        │
└─────────────────────────────┘ └──────────────┬──────────────┘
                                               │
                                               ▼
                                ┌─────────────────────────────┐
                                │       ExecutionPolicy       │
                                ├─────────────────────────────┤
                                │ • CollectErrors:            │
                                │   Ghi lỗi, chạy system sau  │
                                │ • StopPhaseOnError:         │
                                │   Dừng phase hiện tại       │
                                │ • FailFast:                 │
                                │   Hủy bỏ pass ngay lập tức  │
                                └─────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Kịch bản kiểm tra | Kỳ vọng | Thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **HealSystem cập nhật Health** | `Health: 50 -> 100` | `get::<Health>(e) == Some(100)` | **ĐẠT** |
| **Thu thập lỗi FailSystem** | Ghi nhận lỗi có cấu trúc | `report.system_errors[0] == "intentional test failure"` | **ĐẠT** |
| **Chặn ID từ Runtime khác** | Không tìm thấy System | `Err(SystemNotFound)` | **ĐẠT** |
| **Chặn ghi khi chưa xin quyền** | Từ chối truy cập ghi | `report.system_errors[0].1.message.contains("write component")` | **ĐẠT** |
| **Chính sách StopPhaseOnError** | Bỏ qua System phía sau | System `must-not-run` không bao giờ được gọi (reached = 0) | **ĐẠT** |
| **Chính sách FailFast** | Dừng toàn bộ Run Pass | Trả về `Err(SystemExecutionFailed)` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & An Toàn
- **Thời gian thực thi:** `~69 µs`
- **Đánh giá:** `SystemContext` hoạt động như một Firewall bảo vệ `World`. Hệ thống không bao giờ panic khi logic bên ngoài phát sinh lỗi, kiểm soát luồng lỗi 100%.
- **Trạng thái:** **ĐẠT (PASS ✅)**
