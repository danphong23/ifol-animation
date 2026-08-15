# BỘ QUY CHUẨN BÁO CÁO KIỂM THỬ (TEST REPORT STANDARD TEMPLATE)

Tài liệu này định nghĩa **Cấu trúc Báo Cáo Chuẩn Bắt Buộc** cho mọi Test Case trong `ifol-gpu`. Mọi báo cáo kiểm thử đều phải tuân thủ nghiêm ngặt 6 phần này để đảm bảo người dùng có thể đọc hiểu trực quan, kiểm tra đúng/sai và nắm rõ toàn bộ thông số thời gian/hiệu năng.

---

```markdown
# Báo Cáo Kiểm Thử: [MÃ TC] - [Tên Tính Năng Đầy Đủ]

## 1. Ý Nghĩa Bài Toán & Ứng Dụng Thực Tế (What & Why)
- **Vấn đề đồ họa cần giải quyết:** Mô tả rõ nếu xử lý theo cách thông thường (CPU hoặc Pipeline cũ) thì gặp khó khăn gì (nghẽn CPU, tụt FPS, tốn RAM, lỗi hiển thị).
- **Giải pháp kỹ thuật GPU áp dụng:** Nêu rõ kiến trúc giải quyết trên `ifol-gpu` (Compute Shader, RenderGraph Pass, Dual Buffer Ping-Pong, Zero-Copy Vertex/Indirect Draw).
- **Ánh xạ thực tế trong Motion Graphics / Video Editor:** Nêu tính năng tương đương trong After Effects, Premiere, DaVinci Resolve (ví dụ: Puppet Tool, Audio Spectrum, Glow Bloom, Color Scopes, Mask Expansion, Frustum Culling).

---

## 2. Diễn Giải Trực Quan Dữ Liệu & Hướng Dẫn Kiểm Tra Mắt Thường (Visual Inspection)

![Ảnh Kết Xuất](../outputs/desktop/[TÊN_FILE_ANH].png)

### 📐 Bố Cục Không Gian, Hệ Trục & Bảng Chú Giải Màu Sắc:
| Thành phần / Vùng hiển thị | Tọa độ / Ký hiệu | Màu sắc | Kỹ thuật Shader | Ý nghĩa đồ họa thực tế |
| :--- | :--- | :--- | :--- | :--- |
| **Vùng A** | Ví dụ: Nửa trái ($X < 400$) | Vàng kim / Xanh | Vertex/Compute | Trạng thái gốc ban đầu |
| **Vùng B** | Ví dụ: Nửa phải ($X \ge 400$) | Neon Magenta | Post-process | Trạng thái sau biến đổi |

### 👁️ Hướng Dẫn Người Dùng Tự Đánh Giá Đúng/Sai:
- **Dấu hiệu chứng minh thuật toán ĐÚNG:** Nêu 2-3 điểm cụ thể có thể nhìn thấy ngay trên ảnh (ví dụ: đường cong uốn lượn liên tục, hạt chỉ tập trung trong hình tròn bán kính 0.5, không có răng cưa hay vỡ mắt lưới).
- **Dấu hiệu cảnh báo NẾU BỊ LỖI:** Nêu các biểu hiện lỗi điển hình (ví dụ: ảnh bị đen, hạt văng ra khỏi biên, lưới bị xé rách, xuất hiện sọc rác do race condition).

---

## 3. Cấu Trúc Đồ Thị Thực Thi (RenderGraph Pipeline)
- **Đầu vào (Inputs):** Danh sách Texture (độ phân giải, format), Storage Buffer (kích thước, bytes), Uniforms.
- **Chuỗi Pass (Pass Flow):**
  1. `Pass 1 (Compute)`: Tên pipeline, bind groups, số workgroups.
  2. `Pass 2 (Render/Blit)`: Target attachment, blend state, topology, vertex/indirect buffer.
- **Đầu ra (Output Target):** Độ phân giải và định dạng Texture đích.

---

## 4. Đo Lường Hiệu Năng & Thời Gian Thực Thi (Detailed Performance Timings)

### ⏱️ Bảng Phân Rã Thời Gian (Execution Breakdown):
| Hạng mục thực thi | Lần đầu (Cold Start) | Lần sau (Warm / Cached) | Đơn vị đo |
| :--- | :--- | :--- | :--- |
| **Thời gian Chuẩn bị (CPU Graph Build Overhead)** | `X.XX` ms | `X.XX` µs | ms / µs |
| **Thời gian Compute Pass (Tính toán GPU)** | `X.XX` ms | `X.XX` µs | ms / µs |
| **Thời gian Render Pass (Rasterization & Vẽ)** | `X.XX` ms | `X.XX` µs | ms / µs |
| **Tổng Độ Trễ Khung Hình (Total GPU Latency)** | `X.XX` ms | `X.XX` ms | ms |
| **Tốc Độ Khung Hình Tương Đương (Equivalent FPS)**| `XXX` FPS | `XXXX` FPS | FPS |

### ⚙️ Thông Số Phần Cứng & Điều Phối GPU (GPU Dispatch Metrics):
- **Khối lượng dữ liệu xử lý:** Số lượng hạt (Particles) / Vertices / Pixels.
- **Cấu hình Workgroup (Compute):** Kích thước thread group `[X, Y, Z]`.
- **Số lượng Workgroups điều phối:** Dispatch `[Gx, Gy, Gz]`.
- **Dung lượng VRAM chiếm dụng:** Bộ nhớ Buffer / Storage Texture sử dụng.

---

## 5. Xác Thực Tính Toàn Vẹn Số Học & Ràng Buộc An Toàn (Verification Check)
- **Phương pháp đối chiếu:** (Async GPU Readback / Sanity Loop / Shader Validation).
- **Tỷ lệ bảo toàn dữ liệu:** `100.0%` (Không có giá trị NaN, Infinite, Out-of-bounds).
- **Trạng thái:** **PASSED** (Mô phỏng và kết xuất đồ họa đạt chuẩn thiết kế).

---

## 6. Khả Năng Tương Thích & Đa Nền Tảng (Cross-Platform Status)
- **Desktop (Tauri/wgpu - Vulkan/DX12/Metal):** Hoạt động ổn định (Passed).
- **Web (WASM/WebGPU):** *(Sẵn sàng tích hợp khi chạy trên Web)*.
- **Đánh giá tổng quan:** Đạt 100% chuẩn hợp đồng kiến trúc `ifol-gpu`.
```
