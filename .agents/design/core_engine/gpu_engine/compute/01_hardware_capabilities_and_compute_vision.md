# 06. Toàn Cảnh Năng Lực Phần Cứng & Tầm Nhìn Mở Rộng

Để đảm bảo `ifol-gpu` không bao giờ bị lỗi thời và có thể đáp ứng những tham vọng vĩ đại như "Đưa ECS lên chạy thẳng trên GPU", chúng ta phải lập bản đồ toàn bộ các tính năng vật lý mà Card Màn Hình (Hardware) cung cấp thông qua thư viện `wgpu`.

Dưới đây là 3 khối năng lực khổng lồ của GPU Hardware và cách kiến trúc của chúng ta sẽ hấp thụ chúng trong tương lai:

---

## 1. Khối Đồ Họa (Render Pipeline)
Đây là phần cứng chuyên biệt để vẽ ra Điểm ảnh (Pixel). Chúng ta đã bao phủ gần như toàn bộ khối này trong các tài liệu trước.

*   **Tính năng Phần cứng:**
    *   **Z-Buffer (Depth Test):** Đã phân tích (Tài liệu 05). Phần cứng tự chặn vẽ đè.
    *   **Stencil Buffer (Masking):** Phần cứng chặn vẽ theo hình dáng (Mask). Rất quan trọng cho Video Editor (Cắt khung hình tròn, ngôi sao).
    *   **Blend State:** Phần cứng pha trộn màu (Alpha, Additive, Multiply).
    *   **MSAA (Multi-Sample Anti-Aliasing):** Phần cứng khử răng cưa tự động ở viền Mesh.
    *   **Instancing:** Phần cứng lặp lại lệnh vẽ hàng vạn lần với 1 lệnh từ CPU.
*   **Giải pháp Mở rộng cho `ifol-gpu`:**
    *   Lõi `ifol-gpu` chỉ cần bổ sung các cờ (flags) tương ứng vào struct `PipelineConfig` (Ví dụ: `stencil_test`, `msaa_samples`).
    *   `ifol-ecs` sẽ tính toán khi nào cần bật cờ Masking và truyền xuống. Kiến trúc cốt lõi (RenderGraph) không bị phá vỡ.

---

## 2. Khối Tính Toán Thuần Túy (Compute Pipeline)
Đây là thứ bạn nhắc tới: *"Chạy buffer trên GPU thay vì CPU để tối ưu hiệu suất"*. 
GPU hiện đại (từ GTX 10 series trở lên) sở hữu hàng ngàn nhân (CUDA cores / Stream Processors). Chúng có thể làm toán song song (GPGPU - General Purpose GPU) độc lập hoàn toàn với việc vẽ màn hình.

*   **Tính năng Phần cứng:**
    *   **Compute Shader:** Các đoạn mã code không vẽ pixel, mà chỉ đọc/ghi dữ liệu mảng.
    *   **Storage Buffers (SSBO):** Khác với Uniform Buffers (nhỏ, chỉ đọc), Storage Buffers có thể lớn hàng Gigabyte và **GPU CÓ QUYỀN GHI VÀO ĐÓ (Read/Write)**.
*   **Chiến lược Tương lai (Đưa ECS lên GPU):**
    *   Thay vì CPU tính toán vị trí của 100.000 hạt vật lý (Particles) mỗi Frame.
    *   `ifol-ecs` sẽ chuyển toàn bộ Component (Tọa độ, Vận tốc) thành mảng byte thô, nhét vào cái `Storage Buffer` đẩy lên VRAM 1 lần duy nhất.
    *   Mỗi Frame, CPU không làm gì cả. CPU chỉ gọi 1 lệnh Compute: `"Ê GPU, lấy thuật toán Vật lý này, chạy cho 100.000 phần tử trong kho Storage kia đi"`. GPU sẽ dùng hàng ngàn nhân để tính toán lại tọa độ trực tiếp trên VRAM.
*   **Giải pháp cho `ifol-gpu`:**
    *   Trong tương lai, chúng ta sẽ tạo thêm một nhánh song song với `RenderGraph`, gọi là **`ComputeGraph`**.
    *   `ComputeGraph` chỉ chứa `ComputeNode` (Kích hoạt Compute Shader) thay vì lệnh Draw. Lõi `ifol-gpu` sẽ có thêm hàm `queue.submit(ComputeEncoder)`.

---

## 3. Khối Tự Trị (GPU-Driven Rendering)
Đây là đỉnh cao của đồ họa máy tính (Cách mà Unreal Engine 5 Nanite đang hoạt động).

*   **Tính năng Phần cứng:**
    *   **Indirect Drawing:** Bình thường, CPU gửi lệnh `Draw(số_lượng_đỉnh)`. Nhưng với Indirect Draw, số lượng đỉnh nằm sẵn trong VRAM của GPU. CPU gọi lệnh: `"GPU, tự đọc lệnh Draw từ VRAM của mày rồi tự vẽ đi"`.
*   **Tầm Nhìn Đỉnh Cao:**
    *   Kết hợp **Compute Pipeline** (Phần 2) và **Indirect Drawing** (Phần 3): 
    *   Bạn đẩy toàn bộ dữ liệu ECS lên GPU. Compute Shader tự tính toán xem Entity nào nằm trong Camera (GPU Culling). Sau đó GPU tự viết lệnh Draw vào chính VRAM của nó. Cuối cùng GPU tự ra lệnh cho chính nó vẽ.
    *   **Lúc này, `ifol-ecs` (CPU) gần như thất nghiệp hoàn toàn trong khâu Render!**

---

## TỔNG KẾT RANH GIỚI CPU (ECS) VÀ GPU (CORE)
Dù tương lai có nâng cấp lên Compute hay GPU-Driven, quy tắc bất di bất dịch của chúng ta vẫn là:

1.  **`ifol-gpu` (Sứ giả cấp thấp):** 
    *   Là lớp bọc mỏng (Wrapper) của `wgpu`. 
    *   Chỉ cung cấp các hàm tạo Buffer (Uniform, Storage), tạo Texture, và hàm Đọc Mảng Lệnh (RenderGraph / ComputeGraph). 
    *   Tuyệt đối KHÔNG hiểu logic game. Không hiểu Vector, Hình học.
2.  **`ifol-ecs` (Bộ Não Tối Cao):** 
    *   Quản lý Logic Component. 
    *   Quyết định dữ liệu nào nằm ở CPU, dữ liệu nào sẽ bị đẩy lên VRAM (Storage Buffers) để chạy Compute. 
    *   Lắp ráp cái Cấu hình (Cờ Pipeline, Lệnh Draw, Lệnh Compute) rồi vứt qua tường cho `ifol-gpu` thực thi.

Với bản đồ phần cứng này, kiến trúc lõi của chúng ta ĐÃ BAO PHỦ 100% MỌI TÍNH NĂNG VẬT LÝ MÀ `wgpu` (VÀ PHẦN CỨNG CARD MÀN HÌNH) CUNG CẤP. Dự án hoàn toàn "chống cháy" trước mọi yêu cầu mở rộng trong thập kỷ tới!
