# Vòng Đời Tài Nguyên & Tích Hợp UI (Lifecycle & Integration)

Tài liệu này giải đáp các vấn đề về hiệu suất, quản lý tài nguyên (Cache), và cách kết nối kết quả render cuối cùng ra giao diện người dùng (Editor UI).

---

## 1. Mối Quan Hệ ECS - GPU Engine (Singleton & Multi-Viewport)

*   **Khởi tạo Singleton:** GPU Engine (`ifol-gpu`) được khởi tạo đúng 1 lần duy nhất khi ứng dụng bật lên.
*   **Gửi 1 lần duy nhất mỗi khung hình:** Ở mỗi nhịp đồng hồ, `RenderSystem` (ECS) tổng hợp tất cả các `RenderGraph` (màn hình chính, preview nhỏ) thành một CommandEncoder duy nhất và `submit` **MỘT LẦN DUY NHẤT** xuống GPU.
*   **Chia sẻ Node giữa các Viewport:** Nhờ kiến trúc Arena (`RenderNodePool`), 2 Viewport cùng nhìn vào một Entity/Group sẽ chia sẻ chung `RenderNodeId`. GPU chỉ cần thu âm `RenderBundle` 1 lần, cả 2 Viewport đều tái sử dụng được bundle đó.

---

## 2. Quản Lý File Khác (Video, 3D, v.v.)

*   **GPU Engine mù quáng:** `ifol-gpu` không biết đọc `.mp4` hay `.obj`. GPU Engine chỉ biết đọc mảng byte thô (Raw Pixel / Vertex Buffer).
*   **AssetManager (Tầng ECS/Asset):** Chạy FFmpeg giải mã Video lấy frame RGBA, giải mã file 3D thành mảng Vertex.
*   **Luồng đi:** `AssetManager (Giải mã MP4) -> Ép ra RGBA -> Nạp lên VRAM (TextureHandle) -> ECS gán TextureHandle vào BindGroup của DrawCommand`.

---

## 3. Chiến Lược Thu Dọn Rác (Eviction Strategy)

*   **Chủ Động Từ ECS (Deterministic):** ECS biết khi nào đoạn Video trên Timeline trôi qua. ECS gửi chỉ thị xóa `TextureHandle` khỏi VRAM lập tức.
*   **Bị Động Bằng LRU (Safety Net):** Khi VRAM đầy 90%, `ifol-gpu` dùng LRU Cache tự động giải phóng các Texture không dùng trong N giây qua để chống crash OOM.

---

## 4. Tích Hợp Giao Diện UI (Tauri / Svelte)

GPU Engine hỗ trợ 2 Output Mode thông qua `RenderTarget`:

### Hướng 1: Native Surface (Nhanh Nhất - Viewport Chính)
*   Editor UI (Svelte) xin OS Window Surface Handle.
*   `RenderRequestComponent` cài `output_target = RenderTarget::Screen`.
*   GPU Engine vẽ trực tiếp lên Surface của hệ điều hành với độ trễ bằng 0.

### Hướng 2: Offscreen Texture (Preview / Embedded Panel)
*   `RenderRequestComponent` cài `output_target = RenderTarget::Offscreen { color: TextureHandle(P), .. }`.
*   GPU Engine vẽ kết quả ra `TextureHandle(P)`.
*   Svelte UI đọc mảng byte hoặc dùng WebGL/WebGPU shared texture để hiển thị trong thẻ `<canvas>`.
