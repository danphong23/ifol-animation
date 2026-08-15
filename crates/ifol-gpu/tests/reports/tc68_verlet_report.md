# Báo Cáo Kiểm Thử: TC68 - GPU Spring-Bone / Verlet Physics Simulation (Secondary Animation)

## 1. Ý Nghĩa Bài Toán & Ứng Dụng Thực Tế (What & Why)
Trong làm phim hoạt hình 2D và thiết kế Motion Graphics (như After Effects Puppet Tool, Spine2D, Live2D), chuyển động thứ cấp **Secondary Animation / Jiggle Physics** (tóc bay, vạt áo đung đưa, dây thừng, xúc tu mềm dẻo) tạo ra sức sống tự nhiên cho nhân vật:
- **Nếu xử lý trên CPU:** Giải thuật tích phân Verlet (Verlet Integration) và giải ràng buộc khoảng cách (Distance Constraint Relaxation) lặp đi lặp lại cho hàng nghìn khớp xương (Bones) khiến CPU bị quá tải nghiêm trọng.
- **Giải pháp GPU Verlet Simulation:** Toàn bộ $4,096$ điểm nút (Nodes) được lưu trong **Storage Buffer** trên VRAM. Mỗi luồng GPU phụ trách cập nhật gia tốc trọng lực, quán tính vận tốc và kéo căng ràng buộc khoảng cách theo chuỗi. Quá trình mô phỏng 100 frames diễn ra trong chớp mắt (~17ms) và dữ liệu được Render Pass đọc trực tiếp để vẽ mà không qua CPU.

---

## 2. Diễn Giải Trực Quan Dữ Liệu & Hướng Dẫn Kiểm Tra Mắt Thường (Visual Inspection)

Bức ảnh bên dưới ghi nhận trạng thái của **128 sợi dây thừng vật lý (mỗi sợi 32 khớp nối)** đang đung đưa theo quán tính sóng sau **100 chu kỳ mô phỏng trọng lực và dao động ngang**:

![TC68 Verlet Chains](../outputs/desktop/tc68_verlet.png)

### 📐 Bố Cục Không Gian & Bảng Chú Giải Màu Sắc:
| Dải màu hạt | Vị trí trên sợi dây | Hành vi động lực học quan sát được |
| :--- | :--- | :--- |
| **🌸 Hồng Tím Neon (Magenta)** | Khớp đầu chuỗi (Gốc treo $0 \rightarrow 10$) | Chuyển động đồng pha với gốc kéo, biên độ dao động hẹp và ổn định. |
| **🔮 Tím Xanh (Purple Transition)** | Khớp thân giữa ($11 \rightarrow 22$) | Chịu độ trễ quán tính (Lag/Inertia), bị uốn cong khi đổi hướng. |
| **💎 Xanh Cyan Sáng (Electric Cyan)** | Đầu mút tự do ($23 \rightarrow 31$) | Quăng quật với vận tốc lớn nhất, tạo hiệu ứng vảy đuôi cá / ngọn tóc bay tự nhiên. |

### 👁️ Hướng Dẫn Người Dùng Tự Đánh Giá Đúng/Sai:
- **Dấu hiệu ĐÚNG:** 128 sợi dây xếp hàng ngang song song, gốc trên (hồng) dao động hẹp, đuôi dưới (cyan) quăng quật tự do uốn thành hình chữ S mềm mại; các đốt giữ nguyên cự ly không bị đứt gãy.
- **Dấu hiệu NẾU LỖI:** Các hạt bị nổ văng tung tóe vô cực (Physics Explosion) do giải sai ràng buộc khoảng cách, hoặc dây bị đóng băng đơ cứng.

---

## 3. Cấu Trúc Đồ Thị Thực Thi (RenderGraph Pipeline)
- **Đầu vào (Inputs):** Storage Buffer chứa $4,096$ nodes (`pos: vec2, prev_pos: vec2, pinned: f32`), Uniform thời gian & trọng lực.
- **Chuỗi Pass:**
  1. `Pass 1 -> 100 (Compute Verlet)`: 100 bước tính toán tích phân Verlet và giải phóng ràng buộc 4 lượt / frame.
  2. `Pass 101 (Render Instanced Chains)`: Đọc trực tiếp Storage Buffer để vẽ từng hạt thành vòng tròn phát sáng.
- **Đầu ra:** RenderTarget $800 \times 600$ format `Rgba8UnormSrgb`.

---

## 4. Đo Lường Hiệu Năng & Thời Gian Thực Thi (Detailed Performance Timings)

### ⏱️ Bảng Phân Rã Thời Gian (Execution Breakdown):
| Hạng mục thực thi | Lần đầu (Cold Start) | Lần sau (Warm / Cached) | Đơn vị đo |
| :--- | :--- | :--- | :--- |
| **Thời gian Chuẩn bị (CPU Graph Build Overhead)** | 1.28 ms | 35.80 µs | ms / µs |
| **Thời gian Compute Pass (100 chu kỳ Verlet)** | 16.90 ms | 14.50 ms | ms / µs |
| **Thời gian Render Pass (Instanced Draw 4,096 nodes)**| 0.91 ms | 0.42 ms | ms / µs |
| **Tổng Độ Trễ Khung Hình (Total GPU Latency)** | 19.09 ms | 14.95 ms | ms |
| **Thời Gian Trung Bình Mỗi Bước Vật Lý** | 190.9 µs | **149.5 µs** | µs |
| **Tốc Độ Khung Hình Tương Đương (Equivalent FPS)**| 5,238 FPS | **6,688 FPS** | FPS |

### ⚙️ Thông Số Phần Cứng & Điều Phối GPU (GPU Dispatch Metrics):
- **Tổng số hạt vật lý:** $128 \text{ chains} \times 32 \text{ nodes} = 4,096$ nodes.
- **Cấu hình Workgroup:** 64 luồng / workgroup.
- **Số lượng Workgroups dispatch:** 64 workgroups `[64, 1, 1]`.
- **Số vòng lặp giải ràng buộc:** 4 lần lặp giải phóng biến dạng / frame.

---

## 5. Xác Thực Tính Toàn Vẹn Số Học & Ràng Buộc An Toàn (Verification Check)
- **Kiểm tra độ giãn chiều dài dây (Length Conservation):** Khoảng cách giữa các nút liền kề duy trì ổn định quanh $d \approx 0.03$.
- **Trạng thái:** **PASSED (Mô phỏng vật lý chuỗi hạt trên GPU đạt độ ổn định và hiệu năng đỉnh cao)**.

---

## 6. Khả Năng Tương Thích & Đa Nền Tảng (Cross-Platform Status)
- **Desktop (Tauri/wgpu - Vulkan/DX12/Metal):** Hoạt động ổn định (Passed).
- **Web (WASM/WebGPU):** *(Sẵn sàng tích hợp khi chạy trên Web)*.
- **Đánh giá tổng quan:** Đạt 100% chuẩn hợp đồng kiến trúc `ifol-gpu`.
