# 03. Tối Ưu Hiệu Năng & Bộ Nhớ (Memory & Performance)

GPU Engine chỉ mạnh khi nó xử lý được hàng chục nghìn `DrawCommand` mỗi khung hình mà không làm tràn VRAM hay khựng CPU. Tài liệu này định nghĩa các Kỹ thuật Sống còn.

---

## 1. Zero-Allocation Uniform Ring Buffer

*   **Vấn đề:** Mỗi `DrawCommand` chứa một mảng byte `uniforms` (tọa độ, scale, quay). Nếu mỗi khung hình GPU Engine phải cấp phát bộ nhớ (Malloc/New) rồi xóa đi, phần mềm sẽ bị khựng (Stutter).
*   **Giải pháp (Ring Buffer):**
    *   Lúc khởi động, Engine xin 1 cục VRAM trống rỗng to (VD: 10MB - chứa được hàng triệu ma trận tọa độ).
    *   Mỗi khi đọc 1 `DrawCommand`, nó sao chép mảng `uniforms` nhét vào cục 10MB này, và đẩy con trỏ nhích lên.
    *   Kết thúc 1 Frame, con trỏ tự quay về mức 0.
    *   👉 **Kết quả:** Không có lệnh cấp phát bộ nhớ động nào xảy ra trong Loop render. Mượt mà 144 FPS.

---

## 2. RenderBundle Cache Strategy (Chuẩn Bevy / Chunking)

Mỗi `RenderNode` (`DrawBatch` và `SubGraph`) sở hữu một `bundle: Option<wgpu::RenderBundle>`.

### So sánh 3 chiến lược đặt RenderBundle:

| Chiến lược | Đặc điểm | Đánh giá |
|---|---|---|
| **Bundle per Graph** | 1 Graph = 1 Bundle to | ❌ 1 vật thể nhỏ đổi -> CPU thu âm lại toàn bộ |
| **Bundle per Entity** | 1 Entity = 1 Bundle nhỏ | ❌ Gọi `execute_bundles` 10.000 lần gây overhead GPU |
| **✅ Bundle per Batch (Chunking)** | 1 Batch = 1 nhóm Entity chung tính chất | **TỐI ƯU:** Cân bằng hoàn hảo giữa GPU overhead và CPU update cost |

### Cơ chế Invalidation (Khi nào `is_dirty = true`?)

- **KHÔNG Dirty (Tốc độ 0ms CPU):**
  - **Thay đổi vị trí/xoay/tỉ lệ (Transform Animation):** Nhờ Ring Buffer (Dynamic Offset). Bundle chỉ thu âm con trỏ chỉ vào Ring Buffer. Khi vật thể di chuyển, CPU ghi số mới vào VRAM, con trỏ trong Bundle vẫn đúng -> Bundle KHÔNG bị dirty.
  - **Cập nhật khung hình Video:** Texture Handle giữ nguyên, chỉ có byte VRAM bên trong thay đổi (`write_texture`).
- **CÓ Dirty (Thu âm lại Bundle):**
  - Thêm / Xóa DrawCommand trong Node.
  - Đổi Pipeline (Shader) hoặc đổi Mesh.
  - Đổi cấu trúc BindGroup.

---

## 3. Thuật Toán Trục Xuất Texture (LRU Cache)

*   **Vấn đề:** User đẩy 100 video 4K vào Timeline. VRAM bị đầy (Crash OOM).
*   **Giải pháp (Least Recently Used):**
    *   Mỗi `TextureHandle` có con tem "Khung hình cuối cùng được sử dụng".
    *   Nếu VRAM đạt ngưỡng 90%, Engine quét toàn bộ `TextureHandle`. Những ảnh/video không xuất hiện trong N giây qua sẽ bị xóa khỏi VRAM.

---

## 4. Descriptor-based Texture Cache (Exact Match)

*   **Vấn đề:** Cấp phát và xóa liên tục các Texture ngẫu nhiên sẽ làm phân mảnh bộ nhớ VRAM.
*   **Giải pháp (Exact Match Cache):**
    *   Kế thừa kinh nghiệm từ `ifol-render` cũ, ta dùng `Descriptor-based Cache`.
    *   Mỗi khi ECS cần 1 ảnh trung gian (Offscreen), Engine tạo Key `(Width, Height, Format)`.
    *   Khi Offscreen vẽ xong và bị vứt đi (Idle), nó không bị xóa khỏi VRAM mà đưa vào Pool.
    *   Frame tiếp theo, nếu ECS lại cần 1 ảnh có ĐÚNG `(Width, Height, Format)` đó, Engine móc ngay từ Pool ra dùng lại. Tỉ lệ tái sử dụng (Hit-rate) gần 100%.

---

## 5. Cập Nhật Nóng (Fast Texture Update - Video)

*   Video thực chất là 1 Texture thay đổi nội dung 60 lần/giây.
*   **Giải pháp (In-place Write):** Engine cung cấp hàm `write_texture(handle, byte_array)`. Nó cầm mảng byte màu mới và **chép đè thẳng** lên vùng nhớ VRAM của `TextureHandle` có sẵn. Chi phí khởi tạo vật lý bằng 0.
