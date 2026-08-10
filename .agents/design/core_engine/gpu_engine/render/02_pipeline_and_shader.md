# 02. Pipeline & Shader (Sự Tiến Hóa Thành Shader Graph)

Tài liệu này định nghĩa bản chất của `PipelineHandle` và tầm nhìn chiến lược của dự án về việc xử lý Shader.

---

## 1. Bản Chất Hiện Tại: Shader Code
Một `PipelineHandle` thực chất là một chuỗi mã nguồn WGSL (WebGPU Shading Language) đã được Engine biên dịch và đưa vào GPU.

Ví dụ, khi khởi tạo phần mềm:
```rust
pub struct PipelineConfig {
    pub shader_code: String,       // Mã WGSL
    pub blend_mode: BlendMode,     // Alpha, Additive, Multiply...
    pub depth_test: bool,          // Bật/Tắt kiểm tra Z-Buffer (Sự khác biệt 2D/3D)
}
```

**Z-Buffer (Depth Texture) Lấy Ở Đâu Ra?**
Z-Buffer thực chất là một Tấm ảnh (Texture) chỉ chứa dữ liệu Trắng/Đen (thay vì RGBA). 
*   **Ai quản lý?** Lõi `ifol-gpu` sẽ tự động tạo ra một tấm ảnh `DepthTexture` này mỗi khi khởi tạo hoặc Resize màn hình.
*   **Nạp vào kiểu gì?** Khi `ifol-gpu` bắt đầu một phiên vẽ (`RenderPass`), nó gắn cái `DepthTexture` này vào khe cắm `depth_stencil_attachment`.
*   **Tính toán kiểu gì?** Dựa vào cờ `depth_test = true` trong Pipeline, **Phần Cứng (Card Màn Hình)** sẽ tự động đọc/ghi độ sâu vào tấm ảnh này ở cấp độ Pixel. `ifol-gpu` không phải viết 1 dòng code toán học nào.

## 2. Ranh Giới Trách Nhiệm (Sự Ngu Ngốc Của Core GPU)
Bạn cần ghi nhớ một chân lý: **Lõi `ifol-gpu` cực kỳ ngu ngốc. Nó KHÔNG HỀ BIẾT khái niệm Tọa độ Unit, Tọa độ Pixel, Camera View, hay Blend Mode là gì!**

*   **Toán học (Math) & ECS:** ECS dùng tọa độ Unit (0.0 đến 1.0) hay Pixel? Camera phóng to thu nhỏ thế nào? Tất cả được ECS tính toán và nén lại thành một chuỗi byte ma trận vô nghĩa ném vào `uniforms`.
*   **Lõi GPU (`ifol-gpu`):** Chỉ lấy chuỗi byte đó, ném vào phần cứng.
*   **Shader Code (WGSL):** Đây mới là nơi thực sự quyết định việc biến cụm byte `uniforms` đó thành Pixel thực tế. Chế độ hòa trộn (Blend Mode Screen/Multiply), việc hiệu ứng Blur có bị tràn viền hay không, tất cả được định nghĩa 100% bên trong mã nguồn của Shader.

Nhờ sự phân tách tuyệt đối này, lõi GPU Engine của chúng ta sẽ không bao giờ cần sửa đổi mỗi khi ta thêm một hiệu ứng đồ họa hay một hệ quy chiếu toán học mới.

## 3. Tầm Nhìn Tương Lai: Không Gian 3D (Perspective vs Orthographic)
Chính nhờ sự "ngu ngốc" của lõi `ifol-gpu`, kiến trúc này **sẵn sàng cho 3D 100%** ngay từ Ngày 1.
*   **Chuyển đổi 2D sang 3D như thế nào?** Hoàn toàn phụ thuộc vào hệ thống ECS.
    1.  **Camera:** Nếu ECS xác định Camera là 2D (Orthographic), nó ném ma trận phẳng xuống GPU. Nếu Camera là 3D (Perspective), nó ném ma trận phối cảnh 3D xuống.
    2.  **Pipeline (Z-Buffer):** Khi vẽ 3D, ECS chỉ việc truyền một `PipelineHandle` có bật tính năng `Depth Testing` (Kiểm tra chiều sâu). GPU sẽ tự động kích hoạt tính toán Z-Buffer của phần cứng để biết vật thể nào che vật thể nào.
    3.  **Mesh:** Thay vì truyền lệnh vẽ hình vuông (Quad), ECS truyền lệnh vẽ Hình khối (Mesh) với tọa độ X,Y,Z.
*   **Kết luận:** Môi trường 2D hay 3D không thay đổi dù chỉ 1 dòng code của Core GPU. ECS muốn không gian nào, nó chỉ việc đưa đúng Ma trận, đúng Pipeline, và đúng Mesh. Hệ thống lai (Hybrid 2D/3D) hoàn toàn khả thi trong cùng một RenderGraph!

## 4. Shader Graph
Engine chuyên nghiệp không bắt người dùng viết WGSL.
Trong tương lai, chúng ta sẽ có một **Node-based Shader Editor** (Giống Blender, Unreal Engine).

### 2.1. Quá Trình Dịch (Compilation)
Khi người dùng nối Node A (Đổ màu đỏ) vào Node B (Làm mờ):
1. Hệ thống Shader Graph Compiler sẽ "dịch" mạng lưới Node đó ra thành một chuỗi mã nguồn WGSL thuần túy.
2. Nó gọi hàm `engine.register_pipeline(chuỗi_wgsl_vừa_tạo)`.
3. Engine trả về một `PipelineHandle` mới toanh.
4. ECS nhét `PipelineHandle` mới này vào `DrawCommand` của Entity.
👉 **Kết luận:** Lõi `ifol-gpu` không cần thay đổi một dòng code nào. Nó vẫn chỉ nhận một cái `PipelineHandle` mù quáng.

## 3. Quyền Năng Của MCP (AI Agent)
Tương tự như Shader Graph, một AI Agent thông qua Model Context Protocol hoàn toàn có thể:
1. Viết một đoạn text WGSL hoàn toàn mới để tạo ra hiệu ứng đặc biệt mà phần mềm chưa từng có.
2. Bắn lệnh đăng ký đoạn WGSL đó vào Engine (Hot-reloading).
3. ECS gán hiệu ứng đó cho Layer.
Mọi thứ hoạt động Real-time (Thời gian thực) mà không cần khởi động lại phần mềm.
