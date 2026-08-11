# 04. Cầu Nối ECS Sang GPU (ECS to GPU Bridge)

Tài liệu này giải đáp sự băn khoăn về ranh giới: *ECS làm gì và GPU làm gì đối với các thực thể phức tạp như Video hoặc Component Vật liệu (Material)?*

---

## 1. Hệ Thống Xử Lý Video (Video Media System)
*   **GPU Engine:** Không hề biết trên đời có khái niệm "Video". Đối với GPU, nó chỉ là một cái `TextureHandle`.
*   **ECS & Asset Manager:**
    1. Một Entity trong ECS được đánh dấu có `VideoComponent(path: "abc.mp4")`.
    2. Một System chạy ngầm trong ECS (`MediaSystem`) sẽ gọi FFmpeg giải mã file mp4 đó.
    3. Khi Frame số 5 được giải mã xong ra RAM (Mảng Pixel RGB), ECS sẽ gọi lệnh chép đè khối RAM đó vào cái `TextureHandle` của Entity.
    4. Lúc ECS compile ra `RenderGraph`, nó nhét đúng cái `TextureHandle` đó vào lệnh `DrawCommand`.
    5. GPU lấy nó ra vẽ lên màn hình. Video hiển thị mượt mà.

## 2. Hệ Thống Biên Dịch Material (Cấu Trúc Búp Bê Nga - Matryoshka)
Giả sử người dùng gắn **nhiều** Material Component (VD: Glow, rồi tới Blur) vào Entity Video. System của ECS sẽ dịch chuỗi Component này thế nào?

**Tiến trình "Gói Bọc" (Recursive Wrapping):**
1. ECS lấy mảng Node vẽ Video cơ bản ban đầu (Gọi là `Lõi`).
2. ECS phát hiện Material 1 (Glow). Nó tạo một `RenderNode::SubGraph` MỚI:
   - `graph` con: chứa `Lõi` (vẽ Video ra `Offscreen Target 1`).
   - `commands`: chứa `DrawCommand` áp Shader Glow lên `Offscreen Target 1`, rồi vẽ lên target của Graph cha.
3. ECS phát hiện Material 2 (Blur). Nó tạo tiếp một `SubGraph` MỚI thứ 2:
   - `graph` con: chứa toàn bộ SubGraph ở Bước 2 (vẽ ra `Offscreen Target 2`).
   - `commands`: chứa `DrawCommand` áp Shader Blur lên `Offscreen Target 2`.
4. Cứ thế, Entity có bao nhiêu Material thì cái RenderGraph bị bọc lại bấy nhiêu lớp SubGraph cha. Nhờ đó, hiệu ứng được xếp chồng lên nhau chuẩn xác theo thứ tự Material.

**Ai Quyết Định Kích Thước Của Offscreen Target? (Sự Ngu Ngốc Của GPU)**
Nhiều người sẽ thắc mắc: *"Nếu chỉ dùng kích thước gốc của Video/Ảnh, thì việc truyền tham số W, H cho GPU có dư thừa không?"*. Trả lời: **Hoàn toàn KHÔNG! Tham số W, H là sinh tử.**

*   **Vấn đề Tràn Viền (Padding cho Shader):** Giả sử bạn có một bức ảnh gốc 500x500. Bạn gắn hiệu ứng **Blur 50px**. Nếu GPU tự động lấy 500x500, hiệu ứng Blur ở rìa bức ảnh sẽ bị cắt cụt (Clipped).
*   **ECS Giải Quyết:** ECS cực kỳ thông minh. Nó biết ảnh gốc 500x500, nhưng vì có Blur 50px, nó tính toán ra cần một khung bao (Bounding Box) mở rộng là 600x600. Nó ra lệnh cho GPU: `RenderTarget::Offscreen { w: 600, h: 600 }`. 
*   **GPU Nhắm Mắt Làm Theo:** GPU mù quáng cấp phát một cái ảnh rỗng 600x600, ném cho Shader Blur chạy. Hiệu ứng mờ tràn ra ngoài viền ảnh 50px một cách tuyệt đẹp.
*   **Dán Lên Màn Hình:** Cuối cùng, kết quả 600x600 này được ném vào lệnh `DrawCommand` chót. Lệnh DrawCommand này chứa một ma trận tọa độ (Uniforms) đã được ECS tính toán sẵn, nhằm thu nhỏ (Scale) và dịch chuyển cái ảnh 600x600 đó về đúng vị trí pixel vật lý mà người dùng đang nhìn thấy trên màn hình Editor.

## 3. Phân Chia Vai Trò Tuyệt Đối

### A. `ifol-ecs` (The Brain - Kẻ Thông Minh)
ECS sở hữu toàn bộ logic của phần mềm. Nó phải làm những việc sau trước khi gọi GPU:
1.  **Quản lý Mesh & Hình Học:** Khởi tạo Mesh mặc định (Quad 1x1) hoặc tính toán thuật toán tạo ra Mesh đặc biệt (Uốn cong, 3D). Đẩy Mesh lên GPU và lưu giữ `MeshHandle`.
2.  **Toán học Không Gian:** Tính toán ma trận Camera, ma trận vị trí (Transform), tỷ lệ phóng to (Scale).
3.  **Toán học Hiệu Ứng (Padding & UV):** Tính toán độ phình ra của Bounding Box nếu dùng Blur/Glow. Tính toán `UV_Offset` và `UV_Scale` nếu dùng Sprite Sheet.
4.  **Culling & Gom nhóm:** Vứt bỏ các Entity ngoài màn hình (Frustum Culling). Chia làm 2 mảng Đục (Opaque - Bật Z-Buffer) và Trong Suốt (Transparent - Tắt Z-Buffer).
5.  **Sắp xếp (Sorting):** Mảng Đục xếp Gần -> Xa (để tối ưu Early-Z). Mảng Trong xếp Xa -> Gần (để pha trộn Alpha chuẩn).
6.  **Đóng gói RenderGraph:** Dịch tất cả thông tin trên thành một đồ thị `RenderGraph` (chỉ toàn ID, Handle và mảng lệnh) để ném xuống cho GPU.

## 4. Hệ Thống Camera & Hậu Kỳ (Placeholder Injection)
Camera trong Engine không chỉ là một cái ma trận View/Projection, mà còn là nơi xử lý đồ họa tổng hợp (Post-processing).

**Cơ chế Nhét Đồ Thị (Placeholder Injection):**
1. Ở Phase khởi tạo, `CameraComponent` sẽ sinh ra một RenderGraph khung. Bên trong Graph khung này, nó tạo sẵn một **Khu vực rỗng (Placeholder Node)**.
2. Tại các Phase duyệt Entity thông thường, ECS đi thu thập RenderGraph của mọi Layer/Entity (Video, Hình ảnh, Text) nằm trong tầm nhìn của Camera đó.
3. Ở Phase cuối cùng (Trước khi quăng xuống GPU), `CameraSystem` sẽ tổng hợp toàn bộ các Graph rời rạc của Entity thành một Graph con hoàn chỉnh (Scene Graph).
4. `CameraSystem` đem cái Scene Graph đó **nhét thẳng vào** cái Placeholder rỗng ở Bước 1. 
5. Lúc này, Camera hoàn toàn có thể áp dụng các Node hiệu ứng hậu kỳ (Glow toàn màn hình, Color Grading) đè lên cái Placeholder đó (Tương tự như cách Material bọc Entity).

👉 **Bản ngã của Kiến trúc:** `ifol-gpu` cực kỳ ngu ngốc nhưng tốc độ siêu phàm. `ifol-ecs` cực kỳ thông minh, thao túng cấu trúc cây (Bọc Node, Nhét Node) để tạo ra mọi quy luật vật lý và quang học phức tạp.
