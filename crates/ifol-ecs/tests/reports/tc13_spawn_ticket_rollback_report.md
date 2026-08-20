# Báo Cáo Kiểm Thử: TC13 - Lệnh Trì Hoãn, SpawnTicket & Rollback Khi Thất Bại (Commands Buffer)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice13_commands.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice13_commands.rs)  
> **Module liên quan:** [`src/system/commands.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/commands.rs), [`src/system/context.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/context.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC13` (Slice 13)
- **Tên:** Khớp nối `SpawnTicket` trong cùng Command Buffer, từ chối lệnh lỗi không phát lại và hủy lệnh tự động khi System thất bại
- **Mục tiêu kiểm thử:**
  1. Kiểm tra thứ tự khớp nối `SpawnTicket`: Lệnh `insert` dùng `SpawnTicket` sinh bởi `spawn` trong cùng buffer được giải quyết chính xác khi `apply()`.
  2. Kiểm tra lệnh gặp lỗi: Khi gặp lỗi `EntityNotFound`, buffer lập tức hủy bỏ các hành động phía sau (`commands.is_empty() == true`), không để lại trạng thái dang dở.
  3. Kiểm tra System vi phạm quyền ghi khi gọi `ctx.commands().insert()` $\rightarrow$ Bị chặn ngay từ tầng `SystemCommands`.
  4. Kiểm tra Rollback khi System thất bại: System đã đẩy lệnh vào `commands()` nhưng cuối hàm trả về `Err(SystemError)` $\rightarrow$ Toàn bộ lệnh đã đưa vào hàng đợi bị hủy bỏ, `commands_processed == 0`.

---

## 2. Sơ Đồ Trực Quan Quá Trình Khớp Nối SpawnTicket & Rollback Giao Dịch

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ TRƯỜNG HỢP 1: THÀNH CÔNG (SPAWNTICKET RESOLUTION)                                      │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ 1. ctx.commands().spawn()                          ──> Nhận SpawnTicket #0             │
│ 2. ctx.commands().insert(Ticket #0, Pos(4.0, 5.0)) ──> Queue: [Spawn(#0), Insert(#0)]  │
│ 3. System return Ok(())                            ──> Kích hoạt Apply tại Safe Point  │
│ 4. World.spawn() -> EntityId(1v1)                  ──> Ánh xạ Ticket #0 = EntityId(1v1)│
│ 5. World.insert(EntityId(1v1), Pos(4.0, 5.0))      ──> ✅ KHỞI TẠO HOÀN HẢO            │
└────────────────────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────────────────────┐
│ TRƯỜNG HỢP 2: THẤT BẠI (AUTOMATIC TRANSACTION ROLLBACK)                                │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ 1. ctx.commands().insert(WORLD, Health(2))         ──> Đã đưa vào buffer chờ           │
│ 2. System return Err(SystemError)                  ──> Phát hiện lỗi thực thi!         │
│ 3. Runtime kích hoạt: commands.clear()             ──> 🛡️ XÓA TOÀN BỘ LỆNH ĐÃ QUEUE    │
│ 4. commands_processed = 0                          ──> WORLD_ENTITY KHÔNG BỊ BIẾN ĐỔI  │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Kịch bản kiểm tra | Hành vi kỳ vọng | Kết quả thực tế | Đánh giá |
| :--- | :--- | :--- | :---: |
| **Khớp 2 SpawnTickets** | Áp dụng 4 actions thành công | `commands.apply() == Ok(4)`, `Pos.count == 2` | **ĐẠT** |
| **Lệnh trỏ vào Entity chết** | Báo lỗi và xóa sạch buffer | `Err(EntityNotFound)`, `commands.is_empty() == true` | **ĐẠT** |
| **Chặn lệnh không có quyền** | Báo lỗi vi phạm hợp đồng ghi | `report.system_errors.len() == 1`, không mutate | **ĐẠT** |
| **Rollback khi System lỗi** | Hủy toàn bộ lệnh đã queue | `commands_processed == 0`, `Health == None` | **ĐẠT** |
| **Spawn và gán Component** | Sinh 1 entity hoàn chỉnh | `runtime.query::<&Position>().count() == 1` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & Tính Toàn Vẹn
- **Thời gian thực thi:** `~37 µs`
- **Đánh giá:** Mô hình `SpawnTicket` cho phép khởi tạo thực thể phức tạp nhiều component trong cùng 1 pass một cách tất định và an toàn giao dịch (Transactional Atomicity).
- **Trạng thái:** **ĐẠT (PASS ✅)**
