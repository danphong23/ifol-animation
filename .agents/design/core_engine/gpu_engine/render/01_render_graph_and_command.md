# 01. Kiến trúc Cây Gia Phả & Lệnh Vẽ (Render Graph & Command)

Tài liệu này định nghĩa cấu trúc dữ liệu cốt lõi của `ifol-gpu`. Thiết kế này tuân thủ nguyên tắc: **Đệ quy vô hạn**, **Thứ tự mảng là tuyệt đối (Z-Index)**, **Định danh qua Handle**, và **Single Submission**.

## 1. Thiết kế Giao tiếp CPU-GPU (Single Submission)
Đây là bộ mặt của lõi kiến trúc. Để tránh thắt cổ chai "Ping-Pong" giữa CPU và GPU, `ifol-gpu` hoạt động theo nguyên tắc **Single Submission (Gửi 1 lần)**:
*   Mỗi khung hình, CPU (`ifol-ecs`) sẽ tính toán toán học, gom nhóm Opaque (Đục), Transparent (Trong suốt), và xây dựng toàn bộ đồ thị `RenderGraph`.
*   GPU Engine duyệt qua `RenderGraph` này, ghi lại MỌI LỆNH VẼ vào một cái giỏ `wgpu::CommandEncoder`.
*   Chỉ đến cuối cùng, CPU mới ném toàn bộ cái giỏ này xuống GPU (`queue.submit`) MỘT LẦN DUY NHẤT. Dù có vẽ Opaque trước rồi Mờ sau, GPU cũng tự giải quyết trong phần cứng chứ không cần CPU gọi lệnh lần 2.

---

## 2. Hệ Thống Định Danh (Handles)
GPU Engine không quản lý tài nguyên bằng `String` tự do. Nó cấp phát và quản lý qua các cấu trúc an toàn (Strongly Typed Handles).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64); // ID của Shader Code

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64); // ID của Ảnh/Video/Offscreen trên VRAM
```

## 2. Nút Thực Thi (Render Node)
Bản chất một khung hình (Frame) là một danh sách các Nút. 
Một Nút có thể là một Lệnh vẽ (Draw) hoặc một Nhánh đệ quy (SubGraph).

```rust
#[derive(Debug, Clone)]
pub enum RenderNode {
    /// Gọi một nhánh đệ quy. Kết quả của SubGraph này sẽ in ra một TextureHandle riêng biệt.
    SubGraph(Box<RenderGraph>),
    
    /// Gọi lệnh chạm cọ (Vẽ ra Target của mảng cha).
    Draw(DrawCommand),
}
```

## 3. Lệnh Gọi Vẽ (Draw Command)

```rust
#[derive(Debug, Clone)]
pub struct DrawCommand {
    /// Shader sẽ dùng để vẽ.
    pub pipeline: PipelineHandle,
    
    /// Tham số (Tọa độ, Màu, Opacity). Dữ liệu này sẽ được đổ vào Ring Buffer.
    pub uniforms: Vec<u8>,
    
    /// Vật liệu ảnh/video/kết quả subgraph sẽ nạp vào Shader.
    pub bind_textures: Vec<TextureHandle>,
    
    /// Lưới tọa độ (Mesh/Vertices). Nếu None, tự động dùng hình vuông (Quad 4 đỉnh).
    pub mesh: Option<MeshHandle>,
    
    /// Hỗ trợ Batching (Vẽ N lần một lúc).
    pub instance_count: u32,
}
```

## 4. Khung Hình/Nhánh Vẽ (Render Graph)
Không tồn tại khái niệm `RenderPass` cứng nhắc. Thứ tự trước/sau được quyết định hoàn toàn bởi index trong mảng `nodes`.

```rust
#[derive(Debug, Clone)]
pub enum RenderTarget {
    Screen,                                         // In thẳng ra màn hình
    Offscreen { id: TextureHandle, w: u32, h: u32 },// In ra RAM ảo với kích thước chính xác
}

#[derive(Debug, Clone)]
pub struct RenderGraph {
    /// Nhánh này sẽ vẽ kết quả ra đâu?
    pub target: RenderTarget, 
    
    /// Xóa phông nền (Clear Color) trước khi chạy các node.
    pub clear_color: Option<[f32; 4]>,

    /// Lõi thực thi. Engine đọc tuần tự từ index 0 -> N. 
    /// Node ở index 1 luôn vẽ đè lên Node ở index 0.
    pub nodes: Vec<RenderNode>,
}
```

## 5. Cơ Chế Lồng Nhau & Thứ Tự Thực Thi (Nesting & Order)
Sự bối rối thường nằm ở chỗ: *"Khi nào dùng SubGraph, khi nào dùng Draw, và chúng lồng nhau thế nào?"*. Hãy xem nguyên lý sau:
*   **RenderGraph = Khung vẽ (Canvas).** Mỗi Graph có một đích đến (Target). Mọi Node bên trong nó sẽ vẽ lên cái Target đó.
*   **RenderNode = Cây cọ (Brush).** Mảng `nodes` là thứ tự quẹt cọ (Từ index 0 đến N).

### Ví dụ: Vẽ Cảnh Có Quả Cầu Phép Thuật
Giả sử ta cần vẽ: Nhân vật -> Quả cầu phép thuật (phức tạp) -> Hiệu ứng sương mù đè lên cả hai.

```rust
let frame_chinh = RenderGraph {
    target: RenderTarget::Screen,
    nodes: vec![
        // --- INDEX 0: Cầm cọ vẽ Nhân Vật lên Màn Hình ---
        RenderNode::Draw(DrawCommand { pipeline: "draw_image", bind_textures: vec![tex_nhan_vat] }),
        
        // --- INDEX 1: Ra lệnh vẽ Quả Cầu ---
        // Tại đây, ta KHÔNG THỂ vẽ quả cầu trực tiếp lên màn hình, vì quả cầu được tạo
        // từ 2 lớp ánh sáng trộn vào nhau. Nếu trộn thẳng trên màn hình sẽ bị lem màu.
        // Bắt buộc phải khởi tạo một SubGraph (Khung vẽ phụ).
        RenderNode::SubGraph(Box::new(RenderGraph {
            target: RenderTarget::Offscreen(tex_qua_cau_tam), // Vẽ ra 1 cái ảnh ảo trên RAM
            nodes: vec![
                RenderNode::Draw(DrawCommand { /* Lõi lửa đỏ */ }),
                RenderNode::Draw(DrawCommand { /* Hào quang xanh đè lên lõi lửa */ }),
            ]
        })),

        // --- INDEX 2: Dán Quả Cầu lên Màn Hình ---
        // Lúc này SubGraph ở Index 1 đã chạy xong. Kết quả của nó đang nằm trong `tex_qua_cau_tam`.
        // Ta dùng Lệnh Draw để dán cái ảnh đó lên màn hình, đè lên Nhân vật (Index 0).
        RenderNode::Draw(DrawCommand {
            pipeline: "draw_blend",
            bind_textures: vec![tex_qua_cau_tam], // Lấy kết quả của Bước 1 đem ra dùng!
        }),

        // --- INDEX 3: Vẽ Sương Mù ---
        RenderNode::Draw(DrawCommand { /* Vẽ sương mù đè lên tất cả */ }),
    ]
}
```

**Luồng chạy thực tế của GPU:**
1. Nó đứng ở `frame_chinh`, thấy Index 0 là lệnh Draw -> Vẽ nhân vật ra màn hình.
2. Nó thấy Index 1 là `SubGraph` -> Tạm dừng màn hình chính. Nó chui vào SubGraph, thấy có 2 lệnh Draw -> Lần lượt vẽ lửa đỏ, rồi lửa xanh đè lên nhau, in ra cái ảnh ẩn tên là `tex_qua_cau_tam`. Chạy xong, thoát ra.
3. Nó đi tới Index 2, thấy lệnh Draw -> Lấy cái ảnh `tex_qua_cau_tam` vừa tạo xong dán đè lên màn hình.
4. Tới Index 3, lấy sương mù dán đè lên tất cả. Màn hình hoàn tất!

**Tóm lại:** Lồng nhau (`SubGraph`) sinh ra khi bạn cần tính toán một cụm hình ảnh phức tạp ở **bên ngoài màn hình chính**, sau đó gom kết quả của cụm đó thành 1 tấm ảnh duy nhất để vẽ tiếp ở Graph Cha. Thứ tự mảng (List) quy định thằng nào vẽ trước, thằng nào vẽ sau (Z-Index).
