# 05. Cơ Chế Z-Buffer & Kiến Trúc Uniforms

Tài liệu này giải thích hai khái niệm đồ họa cốt lõi nhất gây hiểu lầm giữa CPU (`ifol-ecs`) và GPU (`ifol-gpu`): Sự thay đổi luật vẽ đè của Z-Buffer và Luồng đi của dữ liệu Uniform.

---

## 1. Luật Vẽ Đè & Quyền Lực Của Z-Buffer

Theo nguyên lý cơ bản của mảng (Array): Lõi `ifol-gpu` luôn chạy vòng lặp từ `Index 0` đến `Index N`. **Cái chạy sau luôn có xu hướng vẽ ĐÈ LÊN cái chạy trước.**

Tuy nhiên, `ifol-gpu` có 2 chế độ render được điều khiển bởi `Pipeline`: **TẮT Z-Buffer** và **BẬT Z-Buffer**. Sự khác biệt này làm thay đổi hoàn toàn cục diện đồ họa.

### Chế độ 1: TẮT Z-Buffer (Dành cho vật Trong suốt / 2D UI)
*   **Luật:** Kẻ chạy sau sẽ ĐÈ LÊN kẻ chạy trước một cách tuyệt đối (Thuật toán thợ sơn).
*   **Logic của ECS:** Vì kẻ chạy sau sẽ đè kẻ chạy trước, ECS bắt buộc phải sắp xếp mảng từ **XA đến GẦN**.
    *   `Index 0`: Ngọn núi ở Xa.
    *   `Index 1`: Tấm kính ở Gần.
*   **Kết quả:** Ngọn núi vẽ trước. Tấm kính vẽ sau, đè lên ngọn núi, tạo ra hiệu ứng nhìn xuyên thấu chuẩn xác.

### Chế độ 2: BẬT Z-Buffer (Dành cho vật Đục / 3D Opaque)
*   **Luật (Nghịch lý):** Kẻ chạy sau **CỐ GẮNG** đè lên kẻ chạy trước, nhưng bị phần cứng chặn lại nếu nó nằm xa hơn!
*   **Logic của ECS:** Để tận dụng sức mạnh chặn này (tránh GPU tô màu thừa), ECS sắp xếp mảng ngược lại từ **GẦN đến XA**.
    *   `Index 0`: Bức tường (Gần).
    *   `Index 1`: Ngọn núi (Xa).
*   **Sự vi diệu khi GPU chạy 0 -> N:**
    *   GPU vẽ Bức tường (Index 0). Nó lưu độ sâu của bức tường vào Z-Buffer.
    *   GPU vẽ Ngọn núi (Index 1). Ngọn núi (kẻ chạy sau) cố gắng tô màu đè lên bức tường. Nhưng Z-Buffer của phần cứng hét lên: *"Khoan! Bức tường ở gần hơn, ngọn núi ở xa hơn. Cấm tô màu đè lên!"*.
    *   **Kết quả:** Ngọn núi (kẻ chạy sau) KHÔNG THỂ đè lên Bức tường (kẻ chạy trước). GPU lập tức hủy bỏ phép tính tô màu cho phần núi bị che. Hiệu năng được tiết kiệm tối đa (Early-Z Rejection).

> **TÓM LẠI:** Sự khác biệt nằm ở cái `Pipeline` có chứa cờ `DepthTest = true` hay không. Core GPU luôn nhắm mắt chạy 0->N, nhưng Z-Buffer ON chính là "Tấm khiên" chặn đứng luật vẽ đè của những Index chạy sau mà có tọa độ Z ở xa!

---

## 2. Vertex Buffer & Input Assembler (Phần Cứng Đọc Lưới)
Tương tự như Z-Buffer, sự "ngu ngốc" của lõi `ifol-gpu` tiếp tục được thể hiện ở cách nó nạp lưới tọa độ (Mesh).
*   **Vertex Buffer:** Lưới (Mesh) chứa hàng vạn đỉnh tọa độ không nằm trong RAM của CPU, mà nằm trong VRAM của GPU (Vertex Buffer). 
*   **Input Assembler (Phần cứng):** Khi lõi `ifol-gpu` gọi lệnh `Draw(Mesh_ID)`, nó không hề đọc tọa độ các đỉnh. Thay vào đó, Card Màn Hình có một bộ phận điện tử phần cứng là **Input Assembler**. Bộ phận này tự động thò tay vào VRAM, bốc tọa độ Đỉnh đẩy thẳng vào Vertex Shader.
*   **Kết luận:** Phần mềm `ifol-gpu` chỉ cầm Biển số (Handle) đi rao lệnh. Mọi việc bốc vác tọa độ nặng nhọc đều do Chip điện tử (Hardware) tự động hóa 100%.

---

## 3. Kiến Trúc Dữ Liệu Uniforms

Bạn thắc mắc: *"Có phải toàn bộ thông tin World, Entity nằm chung 1 cục? Nó đi kèm RenderGraph hay sao?"*

Câu trả lời là: **KHÔNG.** RenderGraph KHÔNG chứa dữ liệu Uniform khổng lồ. RenderGraph chỉ chứa **CON TRỎ (Pointer / Handle)** chỉ về phía dữ liệu. Dữ liệu thực sự đã được gửi lên VRAM bằng một luồng riêng biệt.

Trong `ifol-gpu`, Dữ liệu Uniform được chia làm **3 Loại (Bind Groups)** để tránh copy dư thừa:

### Tầng 1: Global Uniform (Dùng chung cả Thế giới)
*   **Chứa:** Thời gian (Time), Số Frame, Độ phân giải màn hình, Ma trận Camera (View/Projection).
*   **Cách hoạt động:** Ở đầu mỗi Frame, CPU gói một cục Struct Global này, gửi 1 lần duy nhất lên VRAM của GPU. Tất cả mọi lệnh Draw đều trỏ về cái địa chỉ VRAM duy nhất này để đọc giờ giấc, camera.

### Tầng 2: Material Uniform (Dùng chung cho 1 Hiệu ứng)
*   **Chứa:** `Texture` (Hình ảnh gốc), Bán kính Blur, Cường độ sáng.
*   **Cách hoạt động:** Giả sử bạn có 100 con quái vật xài chung 1 Shader Dạ Quang. CPU chỉ tải cái cấu hình Dạ Quang đó lên VRAM 1 lần. 100 con quái vật sẽ trỏ về dùng chung.

### Tầng 3: Entity Uniform (Dành riêng cho Từng Vật thể)
*   **Chứa:** Ma trận Transform (Vị trí X, Y, Z, Phóng to, Xoay) VÀ **UV Transform (Cắt cúp ảnh/Sprite Sheet)** của riêng từng vật.
*   **Cách hoạt động (Transform & Sprite Sheet):** 
    *   Đúng như bạn phân tích, bản thân tấm ảnh không hiển thị lên màn hình, mà màn hình hiển thị cái Lưới (Mesh). Lưới này sẽ "chụp" lấy một vùng của tấm ảnh thông qua hệ tọa độ **UV Mapping**.
    *   Nếu bạn dùng **Sprite Sheet** (1 tấm ảnh lớn chứa nhiều icon nhỏ), ECS không hề tạo Mesh mới cho từng icon. ECS vẫn dùng cái Mesh Unit Quad (1x1) mặc định, Scale = (1, 1). 
    *   Nhưng trong Entity Uniform, ECS truyền thêm `UV_Offset` và `UV_Scale`. Lõi GPU (Vertex Shader) sẽ dựa vào con số này để "cắt" đúng cái icon trên tấm ảnh gốc đắp lên Mesh.
*   Đây chính là cái `UniformRingBuffer`. Có 10.000 vật thể? CPU sẽ tải một cái Mảng chứa 10.000 cái (Transform + UV) lên VRAM 1 lần ở đầu Frame. 
*   Bên trong `RenderGraph`, cái lệnh Draw của vật thể số 5 chỉ chứa đúng một con số nguyên nhỏ xíu: `Dynamic_Offset = 5`. GPU thấy số 5, nó tự thò tay vào cái Mảng VRAM khổng lồ kia lấy đúng cái (Ma Trận + UV) số 5 ra vẽ.

> **TÓM LẠI:** RenderGraph rất nhẹ, nó chỉ chứa ID (Handle) của Pipeline, ID của Texture, và Offset. Toàn bộ thông tin đồ sộ (World, Transforms) đều được CPU lén lút đẩy thẳng lên VRAM trước khi ném cái Giỏ lệnh (`CommandEncoder`) xuống cho GPU chạy.
