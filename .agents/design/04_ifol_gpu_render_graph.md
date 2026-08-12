# Thiết Kế Cấu Trúc Đồ Thị Render (Render Graph) Của `ifol-gpu`

> **Lưu ý:** Tài liệu này là bản tổng quan cấp cao. Chi tiết cấu trúc dữ liệu chính thức và Master Architecture nằm tại:
> - 👉 [gpu_engine/render/05_render_architecture_v2_master.md](core_engine/gpu_engine/render/05_render_architecture_v2_master.md)
> - 👉 [gpu_engine/render/01_render_graph_and_command.md](core_engine/gpu_engine/render/01_render_graph_and_command.md)

---

## 1. Bản Chất Đệ Quy Của Render Graph

Để hỗ trợ tính năng **Pre-comp (Composition lồng Composition)** giống After Effects, Render Graph của `ifol-animation` có cấu trúc **Cây Đệ Quy**.

Sự lồng nhau này giải quyết bài toán: **Xử lý hiệu ứng/độ mờ trên cả một Group**.
*   **Nguyên nhân:** Khi vẽ một Group nhân vật (Tay, Chân, Thân), nếu hiệu ứng Blur cần dữ liệu của cả cụm nhân vật, ta phải vẽ cụm đó ra một tấm ảnh nháp (Offscreen Texture) trước.
*   **Giải pháp:** ECS gom nhóm thành một `SubGraph`. Engine vẽ `SubGraph` ra Offscreen Texture trước (Phase 1), sau đó lấy tấm ảnh đó làm Input để áp Shader Blur in lên màn hình chính (Phase 2).

---

## 2. Arena Pattern & Cấu Trúc 1 Graph = 1 Pass

*   **Arena Pattern (`RenderNodePool`):** Toàn bộ `RenderNode` sống trong một Pool trung tâm. `RenderGraph` chỉ chứa mảng `node_ids: Vec<RenderNodeId>`.
*   **1 Graph = 1 RenderPass:** Tất cả các Node trong cùng 1 Graph chia sẻ duy nhất 1 GPU RenderPass. Mỗi Node chỉ phát băng ghi âm `pass.execute_bundles(&[node.bundle])` bên trong Pass đó.
*   **Shader Duy Nhất:** Shader chỉ có 1 loại duy nhất (Đọc từ `@binding` -> Ghi ra `@location`). SubGraph tồn tại không phải vì Shader khác loại, mà vì Input Texture cho bước sau chưa tồn tại (cần vẽ ra trước).

---

## 3. RenderBundle Cache Strategy

*   Mỗi `RenderNode` (`DrawBatch` và `SubGraph`) sở hữu `bundle: Option<wgpu::RenderBundle>`.
*   **Di chuyển / Animation (Transform):** KHÔNG làm dirty bundle nhờ Uniform Ring Buffer (Dynamic Offset). Tốc độ CPU $\approx 0\text{ms}$.
*   **Đổi Shader / Đổi Cấu Trúc:** Bật `is_dirty = true` -> CPU thu âm lại Bundle duy nhất cho Node bị ảnh hưởng.

---

## 4. Pipeline Thực Thi 2-Phase On GPU

1. **Phase 1 (Bottom-up):** Đệ quy duyệt cây, phát hiện `SubGraph` -> vẽ tất cả `SubGraph` ra các Offscreen Texture.
2. **Phase 2 (Top-level):** Mở 1 RenderPass duy nhất cho Screen -> phát tất cả `RenderBundle` của các Node con.
3. **Submit:** `queue.submit()` MỘT LẦN DUY NHẤT cho toàn bộ khung hình.
