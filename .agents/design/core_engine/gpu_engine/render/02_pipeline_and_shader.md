# 02. Pipeline & Shader (Ngôn Ngữ Thống Nhất WGSL)

Tài liệu này định nghĩa bản chất của `PipelineHandle`, chi tiết cấu tạo Render Pipeline và nguyên lý xử lý Shader duy nhất trong `ifol-gpu`.

---

## 1. Bản Chất Shader Duy Nhất

Một `PipelineHandle` đại diện cho một chuỗi mã nguồn WGSL (WebGPU Shading Language) đã được biên dịch cùng với các trạng thái nướng cứng (Baked State) của GPU Pipeline.

### Chân lý: Chỉ có 1 loại Shader duy nhất
Đối với GPU phần cứng, **không có sự phân biệt giữa Material Shader và Compositing Shader**. Tất cả Shader đều tuân theo cùng 1 quy tắc hoạt động:

```text
WGSL Fragment Shader:
  ĐỌC từ khe cắm @binding(N)  ──▶  TÍNH TOÁN MATH  ──▶  GHI ra cổng xuất @location(0)
```

- **Ví dụ 1 (Vẽ ảnh `cat.png`):** Shader đọc `cat.png` ở `@binding(0)`, nhân với màu tint, nhả pixel ra `@location(0)`.
- **Ví dụ 2 (Blur nhân vật):** Shader đọc Offscreen Texture ở `@binding(0)`, tính toán mờ xung quanh, nhả pixel ra `@location(0)`.

Shader **hoàn toàn mù quáng** về việc Texture ở `@binding(0)` là ảnh từ ổ cứng hay là kết quả vẽ từ SubGraph con!

---

## 2. Chi Tiết Cấu Tạo Render Pipeline (`PipelineHandle`)

Trong phần cứng đồ họa hiện đại (WebGPU / Vulkan / Metal / DX12), Render Pipeline là một khối đối tượng không thể thay đổi (Immutable State Object). Tất cả các thông số bên dưới được **nướng cứng (Baked)** khi khởi tạo `PipelineHandle`:

```rust
PipelineHandle = {
    // 1. Mã Nguồn Shader (WGSL)
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,

    // 2. Trạng Thái Hòa Trộn Màu (Blend State)
    blend_state: Option<wgpu::BlendState>, // REPLACE, ALPHA_BLENDING, ADDITIVE, MULTIPLY...

    // 3. Trạng Thái Z-Buffer (Depth / Stencil State)
    depth_stencil: Option<wgpu::DepthStencilState>, // Write Enable, Compare Op (Less/Always)

    // 4. Trạng Thái Hình Học (Primitive State)
    topology: wgpu::PrimitiveTopology, // TriangleList, LineList, PointList...
    cull_mode: Option<wgpu::Face>,     // Front, Back, None

    // 5. Cấu Trúc Khung Ổ Cắm (Pipeline Layout)
    bind_group_layouts: Vec<BindGroupLayout>,
}
```

### Tại sao BlendMode không nằm trong DrawCommand?
- `DrawCommand` chỉ mang nhiệm vụ phát lệnh vẽ (mỏng nhẹ, tốc độ cao).
- `BlendState` và `DepthState` tác động trực tiếp vào phần cứng Fixed-Function Rasterizer của GPU nên **bắt buộc phải nằm ở Pipeline**.
- Khi 2 Entity dùng chung 1 Shader WGSL nhưng khác BlendMode (Entity A vẽ đè, Entity B phát sáng Additive), hệ thống sẽ tạo **2 `PipelineHandle` khác nhau** (2 Pipeline Variants) dùng chung mã nguồn WGSL.

---

## 3. Khả Năng Mở Rộng Bên Ngoài (Extensibility Architecture)

`ifol-gpu` là một lõi đồ họa bị động (Agnostic Engine). Nó **không giới hạn danh sách Pipeline hay Shader**. Các tầng bên ngoài (ECS, Material System, Plugin, AI Agent) có thể tự do mở rộng:

1. **Đăng Ký Động (Dynamic Pipeline Registration):**
   Tầng bên ngoài tạo `wgpu::RenderPipeline` tùy ý (bất kỳ blend mode nào, bất kỳ WGSL shader nào), nạp vào `ResourceRegistry.pipelines.insert(handle, pipeline)`.
2. **Hệ Thống Material Variant Cache (ECS Level):**
   Ở tầng ứng dụng/ECS, một `MaterialRegistry` sẽ quản lý việc ghép cặp:
   $$\text{Material} = \text{WGSL Code} + \text{BlendMode} + \text{DepthSettings}$$
   Nếu biến thể Pipeline đã tồn tại trong Cache → Trả về `PipelineHandle` có sẵn. Nếu chưa có → Tạo mới và chèn vào Registry.
3. **Không Cần Sửa Lõi GPU Engine:**
   Lõi GPU `ifol-gpu` chỉ duyệt `DrawCommand.pipeline` và thiết lập `render_pass.set_pipeline(pipe)`. Việc thêm 100 hiệu ứng shader hay 10 chế độ hòa trộn mới hoàn toàn không yêu cầu sửa 1 dòng code nào trong `ifol-gpu`.

---

## 4. Cơ Chế Đọc `@binding` & Ghi `@location`

### Phía WGSL Shader (`.wgsl`):
```wgsl
@group(0) @binding(0) var tex_input: texture_2d<f32>; // Slot 0
@group(0) @binding(1) var tex_sampler: sampler;        // Slot 1

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { // Cổng xuất 0
    let color = textureSample(tex_input, tex_sampler, in.uv);
    return color;
}
```

### Phía CPU (`ifol-gpu` / Rust):
- **ĐỌC (Input):** CPU cắm `TextureHandle` cụ thể vào `BindGroup` slot 0. Cho dù Handle đó trỏ đến file ảnh hay trỏ đến Offscreen Texture do SubGraph vẽ ra, Shader đọc **hệt như nhau**.
- **GHI (Output):** Shader nhả pixel ra `@location(0)`. Khi Compiler mở `RenderPass`, nó kết nối cổng `@location(0)` với `graph.target`. Pixel tự động chảy vào đúng Target.

---

## 5. Nhiều Texture Inputs Trong 1 Shader

Một Shader có thể mở **nhiều ổ cắm `@binding`** để nhận nhiều Texture cùng lúc:

```wgsl
@group(0) @binding(0) var tex_base: texture_2d<f32>; // Ảnh nền
@group(0) @binding(1) var tex_mask: texture_2d<f32>; // Ảnh mặt nạ (Mask)
@group(0) @binding(2) var my_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let base_color = textureSample(tex_base, my_sampler, in.uv);
    let mask_val = textureSample(tex_mask, my_sampler, in.uv).a;
    return base_color * mask_val; // Ghi ra location(0)
}
```

Ứng dụng: Hòa trộn 2 ảnh (Transition), Cắt đè mặt nạ (Masking), PBR 3D Material (Albedo + Normal + Metallic).

---

## 6. Ranh Giới Trách Nhiệm (Lõi GPU Mù Quáng)

*   **Toán học (Math) & ECS:** ECS dùng tọa độ Unit (0.0 đến 1.0) hay Pixel? Camera phóng to thu nhỏ thế nào? Tất cả được ECS tính toán và nén lại thành mảng byte Uniforms ném vào `bind_groups`.
*   **Lõi GPU (`ifol-gpu`):** Chỉ lấy mảng byte đó, nạp vào GPU.
*   **Shader Code (WGSL):** Quyết định việc biến cụm byte Uniforms đó thành Pixel thực tế.

---

## 7. Shader Graph & Quyền Năng Của MCP (AI Agent)

### Shader Graph Editor
Trong tương lai, Node-based Shader Editor sẽ dịch các mạng lưới Node của người dùng thành mã nguồn WGSL thuần túy, đăng ký với Engine và nhét `PipelineHandle` vào `DrawCommand`.

### AI Agent (MCP)
Thông qua Model Context Protocol, AI Agent có thể:
1. Viết một đoạn text WGSL hoàn toàn mới để tạo ra hiệu ứng đặc biệt.
2. Đăng ký đoạn WGSL đó vào Engine real-time (Hot-reloading).
3. ECS gán `PipelineHandle` đó cho Layer/Entity.
