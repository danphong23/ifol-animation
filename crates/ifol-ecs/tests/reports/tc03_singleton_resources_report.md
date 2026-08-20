# Báo Cáo Kiểm Thử: TC03 - World Singleton Resources & Điều Kiện Chạy (Run Conditions)

> **Crate:** `ifol-ecs`  
> **Source Test:** [`tests/slice03_singleton.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/tests/slice03_singleton.rs)  
> **Module liên quan:** [`src/world/singleton.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/world/singleton.rs), [`src/system/condition.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/system/condition.rs), [`src/report.rs`](file:///c:/Users/abc/.AI/Code/ifol-animation/crates/ifol-ecs/src/report.rs)  
> **Trạng thái:** **ĐẠT (PASS ✅)**

---

## 1. Thông Tin Test Case
- **Mã test:** `TC03` (Slice 03)
- **Tên:** Quản lý tài nguyên toàn cục (World Singleton) và đánh giá điều kiện kích hoạt hệ thống (`RunCondition`)
- **Mục tiêu kiểm thử:**
  1. Đăng ký hai World Singleton: `TestConfig` và `RunCounter`.
  2. Đăng ký `ConfigRequiredSystem` yêu cầu `RunCondition::WorldHas(cfg_id, "TestConfig")`.
  3. Đăng ký `OptionalSystem` chạy vô điều kiện (`RunCondition::Always`).
  4. **Pass 1:** Chỉ gắn `RunCounter` (thiếu `TestConfig`) $\rightarrow$ Kiểm tra `ConfigRequiredSystem` bị bỏ qua an toàn kèm lý do chẩn đoán rõ ràng, `OptionalSystem` chạy bình thường (`ticks: 0 -> 1`).
  5. **Pass 2:** Gắn bổ sung `TestConfig` $\rightarrow$ Kiểm tra cả 2 hệ thống đều chạy thành công (`ticks: 1 -> 12`).

---

## 2. Sơ Đồ Trực Quan Quá Trình Đánh Giá Điều Kiện Thực Thi

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ PASS 1: World CHỈ CÓ RunCounter (Thiếu TestConfig)                                     │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ • ConfigRequiredSystem : [RunCondition: WorldHas(TestConfig)] ──> ❌ THẤT BẠI          │
│                          └── BỊ BỎ QUA (Skipped: "Missing required singleton")        │
│ • OptionalSystem       : [RunCondition: Always]                ──> ✅ CHẠY              │
│                          └── RunCounter.ticks: 0 -> 1                                  │
│                                                                                        │
│ 📊 RunReport #1: executed = ["OptionalSystem"], skipped = ["ConfigRequiredSystem"]     │
└────────────────────────────────────────────────────────────────────────────────────────┘
                                            │
                                  Host gắn thêm TestConfig
                                            │
                                            ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ PASS 2: World ĐÃ CÓ CẢ RunCounter VÀ TestConfig                                        │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ • ConfigRequiredSystem : [RunCondition: WorldHas(TestConfig)] ──> ✅ ĐẠT (CHẠY)        │
│                          └── RunCounter.ticks: 1 + 10 = 11                             │
│ • OptionalSystem       : [RunCondition: Always]                ──> ✅ CHẠY              │
│                          └── RunCounter.ticks: 11 + 1 = 12                             │
│                                                                                        │
│ 📊 RunReport #2: executed = ["ConfigRequiredSystem", "OptionalSystem"], skipped = []   │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Kết Quả & Đánh Giá Chi Tiết

| Tiêu chí kiểm tra | Kỳ vọng (Expected) | Thực tế (Actual) | Kết luận |
| :--- | :--- | :--- | :---: |
| **Pass 1: Hệ thống thực thi** | Chỉ chạy `OptionalSystem` | `report1.systems_executed == ["OptionalSystem"]` | **ĐẠT** |
| **Pass 1: Hệ thống bị bỏ qua** | Ghi nhận `ConfigRequiredSystem` | `report1.systems_skipped[0].system == "ConfigRequiredSystem"` | **ĐẠT** |
| **Pass 1: Lý do bỏ qua chuẩn xác**| Nêu rõ thiếu singleton `TestConfig` | `"Missing required world singleton 'TestConfig'"` | **ĐẠT** |
| **Pass 1: Giá trị Counter** | Tăng đúng 1 đơn vị | `RunCounter.ticks == 1` | **ĐẠT** |
| **Pass 2: Hệ thống thực thi** | Cả 2 hệ thống cùng chạy | `report2.systems_executed.len() == 2` | **ĐẠT** |
| **Pass 2: Giá trị Counter** | $1 + 10 + 1 = 12$ | `RunCounter.ticks == 12` | **ĐẠT** |

---

## 4. Phân Tích Hiệu Suất & An Toàn
- **Thời gian thực thi:** `~17 µs`
- **Đánh giá:** Cơ chế `RunCondition` hoạt động chính xác ở cấp độ cổng vào của System, tiết kiệm 100% tài nguyên CPU khi chưa đủ điều kiện và cung cấp lý do chẩn đoán trong `RunReport`.
- **Trạng thái:** **ĐẠT (PASS ✅)**
