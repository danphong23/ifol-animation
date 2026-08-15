# Báo Cáo Kiểm Thử: TC62 - 2D Storage Texture Read & Write (Image Processing)

## 1. Ý Nghĩa Bài Toán & Ứng Dụng Thực Tế (What & Why)
Trong dựng video và motion graphics, việc xử lý hình ảnh phức tạp (như phát hiện cạnh Sobel Edge Detection, lọc nhiễu, biến đổi không gian màu, optical flow):
- **Nếu dùng Render Pipeline truyền thống:** Phải render qua vertex/fragment shader và bind Render Target framebuffer cồng kềnh, bị hạn chế bởi việc không thể ghi tùy ý (Random Write Access).
- **Giải pháp Storage Texture Compute:** Cho phép Compute Shader đọc trực tiếp từ Texture nguồn và ghi tùy ý vào bất kỳ tọa độ $(x, y)$ của `texture_storage_2d` mà không cần tam giác hay Rasterizer.
- **Ánh xạ thực tế:** Tương tự hiệu ứng Find Edges / Glow trong After Effects hoặc OpenFX plugins trong DaVinci Resolve.

---

## 2. Diễn Giải Trực Quan Dữ Liệu & Hướng Dẫn Kiểm Tra Mắt Thường (Visual Inspection)

Bức ảnh bên dưới thể hiện bố cục so sánh **Side-by-Side** được tạo ra hoàn toàn bởi Compute Shader:

![TC62 Storage Texture](../outputs/desktop/tc62_storage_texture.png)

### 📐 Bố Cục & Chú Giải Vùng Ảnh:
| Vùng hiển thị | Tọa độ Pixel $X$ | Kỹ thuật Compute thực hiện | Mô tả trực quan |
| :--- | :--- | :--- | :--- |
| **🖼️ Nửa Trái (Left Half)** | $X < 400$ | `textureLoad` $\rightarrow$ `textureStore` | **Ảnh gốc ban đầu:** Hiển thị hình ảnh nhân vật nguyên bản (Raw Source). |
| **⚡ Vạch Ngăn Cách (Divider)** | $398 \le X \le 402$ | `textureStore(vec4(0.0, 0.95, 1.0, 1.0))` | **Vạch chia Cyan:** Đường ranh giới phân tách 2 chế độ xử lý. |
| **🎨 Nửa Phải (Right Half)** | $X \ge 400$ | Ma trận chập Sobel $3 \times 3$ + Inverted Neon Edge | **Ảnh đã xử lý:** Toàn bộ viền nhân vật được bóc tách và phát quang màu Magenta-Gold trên nền tối. |

### 👁️ Hướng Dẫn Người Dùng Tự Đánh Giá Đúng/Sai:
- **Dấu hiệu ĐÚNG:** Nửa bên trái ảnh hiển thị nhân vật bình thường, có một vạch xanh cyan đứng phân đôi ở giữa ($X=400$), nửa bên phải viền nhân vật sáng màu neon trên nền đen.
- **Dấu hiệu NẾU LỖI:** Nếu toàn bộ ảnh bị đen, vạch chia bị lệch tọa độ hoặc viền sobel bị đứt gãy/vỡ hạt do đọc sai tọa độ lân cận.

---

## 3. Cấu Trúc Đồ Thị Thực Thi (RenderGraph Pipeline)
- **Đầu vào (Inputs):** Texture nguồn RGBA ($800 \times 600$), Uniform tham số Sobel.
- **Chuỗi Pass:**
  1. `Pass 1 (Compute)`: Shader Sobel đọc `TextureView` nguồn và ghi vào `StorageTexture2D` đích.
- **Đầu ra:** Texture đích $800 \times 600$ format `Rgba8Unorm`.

---

## 4. Đo Lường Hiệu Năng & Thời Gian Thực Thi (Detailed Performance Timings)

### ⏱️ Bảng Phân Rã Thời Gian (Execution Breakdown):
| Hạng mục thực thi | Lần đầu (Cold Start) | Lần sau (Warm / Cached) | Đơn vị đo |
| :--- | :--- | :--- | :--- |
| **Thời gian Chuẩn bị (CPU Graph Build Overhead)** | 0.85 ms | 24.30 µs | ms / µs |
| **Thời gian Compute Pass (Tính toán Sobel)** | 16.23 ms | 1.73 ms | ms / µs |
| **Thời gian Render Pass / Blit ra Target** | 0.00 ms *(Zero Rasterizer)* | 0.00 ms *(Zero Rasterizer)* | ms / µs |
| **Tổng Độ Trễ Khung Hình (Total GPU Latency)** | 17.08 ms | 1.75 ms | ms |
| **Tốc Độ Khung Hình Tương Đương (Equivalent FPS)**| 58.5 FPS | **571.4 FPS** | FPS |

### ⚙️ Thông Số Phần Cứng & Điều Phối GPU (GPU Dispatch Metrics):
- **Độ phân giải xử lý:** $800 \times 600$ pixels ($480,000$ điểm ảnh).
- **Cấu hình Thread Group:** 2D Workgroup $16 \times 16$ (256 luồng / workgroup).
- **Số lượng Workgroups dispatch:** $50 \times 38 = 1,900$ workgroups `[50, 38, 1]`.
- **Tổng số luồng GPU thực thi song song:** 486,400 invocations.

---

## 5. Xác Thực Tính Toàn Vẹn Số Học & Ràng Buộc An Toàn (Verification Check)
- **Kiểm tra biên (Boundary Check):** Khi lấy mẫu ma trận Sobel $3 \times 3$ tại các pixel viền mép ảnh ($X=0, Y=0, X=799, Y=599$), shader sử dụng `clamp` tọa độ an toàn, không bị tràn bộ nhớ VRAM.
- **Trạng thái:** **PASSED (Xác thực đọc/ghi Storage Texture 2D thành công 100%)**.

---

## 6. Khả Năng Tương Thích & Đa Nền Tảng (Cross-Platform Status)
- **Desktop (Tauri/wgpu - Vulkan/DX12/Metal):** Hoạt động ổn định (Passed).
- **Web (WASM/WebGPU):** *(Sẵn sàng tích hợp khi chạy trên Web)*.
- **Đánh giá tổng quan:** Đạt 100% chuẩn hợp đồng kiến trúc `ifol-gpu`.
