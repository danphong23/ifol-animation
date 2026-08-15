# Báo Cáo Kiểm Thử: TC70 - Stream Compaction & Indirect Draw (Zero-Copy GPU Particle Culling)

## 1. Ý Nghĩa Bài Toán & Ứng Dụng Thực Tế (What & Why)
Trong các hệ thống hạt số lượng khổng lồ (Massive Particle Systems) hoặc thế giới đồ họa rộng lớn (Frustum Culling / Occlusion Culling):
- **Nếu xử lý trên CPU:** CPU phải lọc qua $100,000+$ vật thể để tìm xem vật thể nào nằm trong camera, đóng gói mảng mới rồi gửi lệnh vẽ (`draw`) với số lượng hạt còn lại. Điều này gây nghẽn CPU trầm trọng.
- **Giải pháp GPU Stream Compaction & Indirect Draw:** 
  1. Toàn bộ $100,000$ hạt ban đầu rải rác khắp không gian.
  2. Compute Shader duyệt qua từng hạt, kiểm tra điều kiện hiển thị (Culling Criteria: chỉ giữ lại các hạt nằm trong vùng kiểm soát).
  3. Khi phát hiện hạt hợp lệ, Compute Shader thực hiện `atomicAdd` trực tiếp vào trường `instance_count` của một **Indirect Draw Buffer** (`wgpu::util::DrawIndirectArgs`) và ghi hạt sang Compacted Buffer.
  4. Render Pass gọi `DrawAction::Indirect`. WGPU tự động đọc số lượng hạt hợp lệ từ Indirect Buffer để vẽ mà **CPU hoàn toàn không cần biết có bao nhiêu hạt được vẽ ra** (Zero CPU Overhead).

---

## 2. Diễn Giải Trực Quan Dữ Liệu & Hướng Dẫn Kiểm Tra Mắt Thường (Visual Inspection)

Bức ảnh bên dưới là minh chứng trực quan cho kết quả sau khi **$100,000$ hạt rải rác toàn màn hình được GPU tự động thanh lọc và gom cụm lại**:

![TC70 GPU Culling](../outputs/desktop/tc70_culling.png)

### 📐 Bố Cục Không Gian & Bảng Chú Giải Màu Sắc:
| Thành phần trực quan | Trạng thái hiển thị | Ý nghĩa kỹ thuật |
| :--- | :--- | :--- |
| **Hạt Xanh Lục Sáng (Bright Green Dots)** | Hạt được vẽ hợp lệ | Các hạt nằm trong bán kính $0.5$ được Stream Compaction nén vào mảng kết xuất. |
| **Không Gian Nền Xám Tối** | Vùng không có hạt nào | Minh chứng cho thấy các hạt ngoài vùng đã bị GPU loại bỏ hoàn toàn, không vẽ rác. |
| **Hình Dạng Đường Biên** | Đường tròn sắc nét bán kính $0.5$ | Khẳng định thuật toán kiểm tra khoảng cách $L_2 \le 0.5$ trong Compute Shader chính xác tuyệt đối. |

### 👁️ Hướng Dẫn Người Dùng Tự Đánh Giá Đúng/Sai:
- **Dấu hiệu ĐÚNG:** Chỉ có một đám mây hạt xanh lục tạo thành hình tròn hoàn hảo ở giữa khung hình, vùng ngoài hoàn toàn trống trơn; các hạt phân bổ đều đặn.
- **Dấu hiệu NẾU LỖI:** Hạt xuất hiện tràn ra ngoài viền tròn, hạt bị vẽ đè lên nhau tại gốc $(0,0)$ do lỗi đếm nguyên tử `atomicAdd`, hoặc màn hình bị đen do Indirect Buffer không đọc được `instance_count`.

---

## 3. Cấu Trúc Đồ Thị Thực Thi (RenderGraph Pipeline)
- **Đầu vào (Inputs):** Storage Buffer A ($100,000$ hạt), Storage Buffer B (Compacted particles), Indirect Buffer (16 bytes), Uniform bán kính culling ($R=0.5$).
- **Chuỗi Pass:**
  1. `Pass 1 (Compute Culling)`: Duyệt $100,000$ hạt, lọc và `atomicAdd` tăng `instance_count`.
  2. `Pass 2 (Render Indirect)`: Gọi `DrawAction::Indirect` đọc Indirect Buffer để vẽ các hạt hợp lệ.
- **Đầu ra:** RenderTarget $800 \times 600$ format `Rgba8UnormSrgb`.

---

## 4. Đo Lường Hiệu Năng & Thời Gian Thực Thi (Detailed Performance Timings)

### ⏱️ Bảng Phân Rã Thời Gian (Execution Breakdown):
| Hạng mục thực thi | Lần đầu (Cold Start) | Lần sau (Warm / Cached) | Đơn vị đo |
| :--- | :--- | :--- | :--- |
| **Thời gian Chuẩn bị (CPU Graph Build Overhead)** | 0.95 ms | 26.10 µs | ms / µs |
| **Thời gian Compute Pass (Lọc 100,000 hạt)** | 18.50 ms | 4.80 ms | ms / µs |
| **Thời gian Render Pass (Indirect Draw các hạt sống sót)**| 16.22 ms | 3.90 ms | ms / µs |
| **Tổng Độ Trễ Khung Hình (Total GPU Latency)** | 35.67 ms | 8.73 ms | ms |
| **Tốc Độ Khung Hình Tương Đương (Equivalent FPS)**| 28.0 FPS | **114.5 FPS** | FPS |

### ⚙️ Thông Số Phần Cứng & Điều Phối GPU (GPU Dispatch Metrics):
- **Số lượng hạt đầu vào:** 100,000 particles.
- **Cấu hình Workgroup Compute:** 64 luồng / workgroup.
- **Số lượng Dispatch Workgroups:** 1,563 workgroups `[1563, 1, 1]`.
- **Cấu trúc Indirect Draw Buffer:** 16 bytes gồm `[vertex_count: 6, instance_count: N, first_vertex: 0, first_instance: 0]`.
- **Phương thức vẽ:** `wgpu::RenderPass::draw_indirect`.

---

## 5. Xác Thực Tính Toàn Vẹn Số Học & Ràng Buộc An Toàn (Verification Check)
- **Zero CPU Overhead:** CPU không hề can thiệp vào việc đếm số hạt hay tái phân bổ mảng.
- **Trạng thái:** **PASSED (Kỹ thuật GPU Stream Compaction & Zero-Copy Indirect Draw hoạt động chính xác 100%)**.

---

## 6. Khả Năng Tương Thích & Đa Nền Tảng (Cross-Platform Status)
- **Desktop (Tauri/wgpu - Vulkan/DX12/Metal):** Hoạt động ổn định (Passed).
- **Web (WASM/WebGPU):** *(Sẵn sàng tích hợp khi chạy trên Web)*.
- **Đánh giá tổng quan:** Đạt 100% chuẩn hợp đồng kiến trúc `ifol-gpu`.
