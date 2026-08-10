# Bản Thiết Kế Định Hướng Kiến Trúc Hệ Thống `ifol-animation`

Tài liệu này đúc kết triết lý tối giản cốt lõi của hệ thống `ifol-animation` được đồng thuận giữa Người phát triển và AI Agent. Mọi thiết kế, triển khai trong tương lai đều phải tuân thủ nghiêm ngặt định hướng này.

---

## 1. Triết Lý Thiết Kế Cốt Lõi: Đơn Giản Hóa Tuyệt Đối

1.  **GPU Chỉ Làm Nhiệm Vụ Vẽ (Mù Lòa Logic):** 
    *   Crate GPU (`ifol-gpu`) là một máy trạng thái vẽ thô phi nghiệp vụ. Nó không biết gì về ECS, Camera, Video, hay Keyframe. Nó chỉ nhận vào một danh sách các lệnh vẽ thô phẳng (`DrawCommand`) chứa VRAM Texture Keys, Vertex data, Uniform parameters và vẽ lên Render Target chỉ định.
2.  **Mọi Logic Gọi Vẽ Nằm Ở ECS (`ifol-ecs`):**
    *   Việc quyết định vẽ cái gì, vẽ như thế nào, vẽ ở đâu hoàn toàn do các System trong ECS điều phối và đóng gói thành chỉ thị vẽ trước khi chuyển cho GPU.
    *   **Render theo đối tượng:** Nếu muốn vẽ trực tiếp 1 Entity (vd: ảnh thô) lên viewport, ECS chỉ cần sinh draw call duy nhất của thực thể đó.
    *   **Render theo Camera:** Hệ thống quét các Entity nằm trong vùng nhìn của Camera, lọc theo `LayerComponent` và `LifespanComponent`, sau đó tạo danh sách draw call tương ứng. Các Camera Entity khác xuất hiện trong viewport chỉ hiển thị dưới dạng khung dây (Gizmo) nếu bật chế độ Editor Layer, tuyệt đối không tự động render đè lên nhau.
    *   **Render đệ quy (Composition):** Trong tương lai, `CompositionComponent` thực chất là một Scene phụ được chạy đệ quy qua chính luồng xử lý của Scene lớn để xuất ra Texture tạm.
3.  **Tách Biệt Phần Mở Rộng:**
    *   Các tính năng như Keyframe nội suy (Animation), Custom Shader phức tạp, Video stream decode qua FFmpeg... **không thuộc về khung xương lõi ban đầu**. Chúng chỉ là các component và system được cắm thêm vào sau khi khung lõi đã hoạt động ổn định và được test kỹ càng.

---

## 2. Các Thành Phần Lõi Của Hệ Thống (Core Elements)

Hệ thống ban đầu sẽ chỉ được hình thành từ 8 yếu tố cốt lõi sau:

1.  **Entity**: Định danh số nguyên độc nhất đại diện cho một thực thể (rỗng).
2.  **Component**: Các túi chứa dữ liệu thuần túy (Pure Data).
3.  **System**: Logic xử lý dữ liệu và lập lịch (Update pipeline).
4.  **Draw Call (`DrawCommand`)**: Chỉ thị vẽ được sinh ra bởi ECS gửi xuống GPU.
5.  **GPU Render (`ifol-gpu`)**: Driver giao tiếp với card màn hình qua wgpu.
6.  **Layer (`LayerComponent`)**: Xác định thứ tự vẽ đè lên nhau của thực thể.
7.  **Thời gian sống (`LifespanComponent`)**: Component bắt buộc duy nhất xác định sự tồn tại của Entity tại thời điểm $t$.
8.  **Scene & Current Time**: Trạng thái tổng quản lý tiến trình playhead hiện tại.

---

## 3. Thứ Tự Ưu Tiên Xây Dựng (Core-Outward Roadmap)

Chúng ta sẽ đi từng bước nhỏ, test chặt chẽ từng bước:

### Bước 1: Thiết lập cấu trúc cơ bản (`ifol-math` & `ifol-gpu` thô)
*   **Mục tiêu:** GPU vẽ được hình thô lên màn hình hoặc lưu ra ảnh PNG từ một danh sách chỉ thị vẽ phẳng mà không cần bất kỳ logic ECS nào.
*   **Unit Test:** Đảm bảo wgpu compile shader builtin cơ bản và vẽ đúng tọa độ.

### Bước 2: Xây dựng bộ khung ECS tối giản (`ifol-ecs`)
*   **Mục tiêu:** Khởi tạo được World chứa Entity, nạp các component cơ bản (`LifespanComponent`, `LayerComponent`, `Transform`).
*   **Unit Test:** Kiểm tra việc lọc Entity tồn tại theo thời gian sống tại mốc thời gian $t$.

### Bước 3: Draw Compiler & Phân cấp (Hierarchy)
*   **Mục tiêu:**
    *   Xây dựng hệ thống phân cấp Cha-Con (`ParentComponent`).
    *   Tính toán ma trận Transform toàn cục (Global Matrix) dựa trên phân cấp cha-con.
    *   Viết hệ thống biên dịch: Duyệt qua các Entity đang sống -> Tính toán vị trí -> Sinh ra danh sách `DrawCommand` thô chuyển cho `ifol-gpu` thực hiện vẽ.

### Bước 4: Tích hợp Host đầu tiên (CLI hoặc MCP thô)
*   **Mục tiêu:** Chạy CLI nhận file JSON chứa thông số Entities đơn giản, render ra 1 frame ảnh tĩnh lưu xuống đĩa.

*(Các phần Camera nâng cao, Video decode, Keyframe Animation sẽ được lập kế hoạch phát triển sau khi hoàn thành 4 bước lõi trên)*
