# ifol-gpu Feature Matrix

Tài liệu này theo dõi các tính năng hiện có của lõi GPU, được cập nhật liên tục bởi AI Agent theo **Feature Tracking Rule**. Các module bên ngoài có thể tham chiếu tài liệu này để biết lõi GPU đang có năng lực gì.

## 1. Khởi Tạo Engine (Initialization) - Đã hoàn thành
- `[x]` **Headless Builder (`GpuEngineBuilder`)**: Hỗ trợ khởi tạo hệ thống GPU mù lòa (không cần Window). Cho phép yêu cầu Backend cụ thể (Vulkan/Metal/DX12/WebGPU).
- `[x]` **Fallback Mechanism**: Tự động lùi về chuẩn đồ họa thấp hơn nếu cấu hình cao hơn không được phần cứng hỗ trợ.
- `[x]` **Hardware Capabilities Scanning (`GpuCapabilities`)**: Bọc lại `wgpu::Limits` thành cấu trúc an toàn. Lấy ra được các cực trị phần cứng (`max_texture_dimension_2d`, `max_bind_groups`, `min_uniform_buffer_offset_alignment`).

## 2. Quản Lý Đồ Thị Render (Arena Pattern & 2-Phase Render Graph) - Đã hoàn thành
- `[x]` **Resource & Node Handles** (`TextureHandle`, `PipelineHandle`, `MeshHandle`, `BindGroupHandle`, `BufferHandle`, `RenderNodeId`): Đóng gói con trỏ tài nguyên và Node thành ID nguyên thủy (u64) nhằm tối ưu bộ nhớ và bảo đảm tính an toàn khi truyền qua Command Bus / ECS.
- `[x]` **Arena Node Pool (`RenderNodePool`)**: Quản lý tập trung toàn bộ `RenderNode` trong Arena HashMap, hỗ trợ $O(1)$ lookup, cờ `is_dirty`, và cho phép multi-viewport chia sẻ chung Node.
- `[x]` **Target-Agnostic Single Shader Architecture**: Shader đọc từ `@binding(N)` và nhả ra `@location(0)`, hoàn toàn mù quáng về nguồn gốc Texture và Target đích đến.
- `[x]` **DrawAction Enum (`Indexed` & `Procedural`)**: Phân tách rạch ròi hành động vẽ theo Mesh trong VRAM hoặc vẽ Procedural Fullscreen Quad bằng Vertex Shader mà không bị hard-code.
- `[x]` **DrawCommand Struct**: Đóng gói `pipeline`, `bind_groups` (slot, handle, dynamic_offsets cho Ring Buffer) và `action`.
- `[x]` **SubGraph & DrawBatch (`RenderNode`)**: Hỗ trợ đồ thị đệ quy (Cây lồng nhau) phục vụ cho Pre-comp, Grouping, Group Alpha/Blur như After Effects.
- `[x]` **3D-Ready RenderGraph**: Tích hợp `depth_stencil: Option<TextureHandle>` và `clear_color` trực tiếp ở cấp Graph, sẵn sàng cho Z-Buffer 3D và 2.5D hybrid rendering.
- `[x]` **2-Phase Compiler (1 Graph = 1 RenderPass)**:
  - **Phase 1 (Bottom-Up)**: Đệ quy xử lý SubGraph con ra Offscreen Texture trước khi mở RenderPass cha.
  - **Phase 2 (Top-Level)**: Mở 1 `wgpu::RenderPass` duy nhất cho Target hiện tại, phát tất cả `RenderBundle` (`execute_bundles`).
- `[x]` **Surface View Binding (`execute_with_surface`)**: Cho phép render trực tiếp ra cửa sổ `winit` thông qua `RenderTarget::Screen`.

## 3. Quản Lý Bộ Nhớ & Performance - Đã hoàn thành
- `[x]` **Uniform Ring Buffer**: Cấp phát động dữ liệu Uniform với cơ chế quay vòng. Tự động tính toán Padding theo giới hạn căn lề chuẩn của phần cứng (`min_uniform_buffer_offset_alignment`).
- `[x]` **RenderBundle Caching (per-Batch Chunking)**: Lưu `wgpu::RenderBundle` trong RenderNode. Transform Animation KHÔNG làm dirty bundle nhờ Dynamic Offset.
- `[x]` **Resource Registry (`IndexFormat` 3D/2D)**: Ánh xạ linh hoạt từ Handle siêu nhẹ ra các thực thể VRAM thực thụ. Mesh hỗ trợ cả `IndexFormat::Uint16` lẫn `Uint32`.
- `[x]` **Texture Utility**: Hàm đọc ảnh từ VRAM về RAM (`save_texture_to_file`) hỗ trợ chụp ảnh màn hình và Snapshot Testing.

---

## Hướng Dẫn Sử Dụng (Usage Examples)

### 1. Khởi Tạo Engine (Headless)

```rust
use ifol_gpu::api::GpuEngineBuilder;

async fn init_gpu() {
    let engine = GpuEngineBuilder::new()
        .build()
        .await
        .expect("Lỗi: Không tìm thấy Card đồ họa tương thích!");

    let caps = engine.capabilities();
    println!("Max Texture Size hỗ trợ: {}", caps.max_texture_dimension_2d);
}
```

### 2. Xây Dựng & Thực Thi Render Graph Với `RenderNodePool` (Arena Pattern)

```rust
use ifol_gpu::render::{
    RenderGraph, RenderNodePool, RenderTarget, ResourceRegistry,
    TextureHandle, PipelineHandle, MeshHandle, BindGroupHandle,
    DrawCommand, DrawAction, RenderGraphExecutor,
};

fn render_frame(engine: &ifol_gpu::api::GpuEngine) {
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();
    let mut pool = RenderNodePool::new();

    // 1. Đăng ký tài nguyên VRAM vào Registry
    // registry.textures.insert(TextureHandle(1), offscreen_view);
    // registry.pipelines.insert(PipelineHandle(10), blur_pipeline);

    // 2. Tạo SubGraph con (vẽ nhân vật ra Offscreen Texture 1024x1024)
    let mut char_subgraph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 1024,
        height: 1024,
    })
    .with_clear_color([0.0, 0.0, 0.0, 0.0]);

    // Thêm các lệnh vẽ bộ phận nhân vật (Tay, Chân, Đầu) vào Pool
    char_subgraph.add_batch(
        &mut pool,
        vec![
            DrawCommand::new(
                PipelineHandle(1),
                DrawAction::Indexed {
                    mesh: MeshHandle(100),
                    index_range: 0..36,
                    instance_range: 0..1,
                },
            ),
        ],
    );

    // 3. Tạo RootGraph chính (Target: Screen) với Z-Buffer cho 3D
    let mut root_graph = RenderGraph::new(RenderTarget::Screen)
        .with_clear_color([0.1, 0.1, 0.1, 1.0])
        .with_depth_stencil(TextureHandle(2));

    // Nhét SubGraph "CharPass" vào RootGraph qua Pool, kèm DrawCommand áp Blur
    root_graph.add_subgraph(
        &mut pool,
        "CharPass",
        char_subgraph,
        vec![
            DrawCommand::new(
                PipelineHandle(10), // Blur pipeline
                DrawAction::Procedural {
                    vertex_count: 6, // 2 triangles for Fullscreen Quad
                    instance_range: 0..1,
                },
            )
            .with_bind_group(0, BindGroupHandle(1), vec![]), // Bind texture offscreen (TextureHandle 1)
        ],
    );

    // 4. Biên dịch 2-Phase (1 Pass per Graph) và nộp xuống GPU 1 lần duy nhất
    let idx = executor.execute(&engine, &registry, &mut pool, &root_graph);
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(idx),
        timeout: None,
    });
}
```

### 3. Cấp Phát Động Uniform Data (Ring Buffer)

```rust
use ifol_gpu::memory::UniformRingBuffer;

fn update_uniforms(engine: &ifol_gpu::api::GpuEngine) {
    let alignment = engine.capabilities().min_uniform_buffer_offset_alignment;
    let mut ring = UniformRingBuffer::new(engine.device(), 1024 * 1024, alignment);

    let my_matrix_data = [1.0f32; 16]; 
    let offset = ring.write(engine.queue(), &my_matrix_data).unwrap();

    // Truyền offset này vào DrawCommand để GPU đọc đúng vị trí
    let _cmd = DrawCommand::new(
        PipelineHandle(1),
        DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 },
    )
    .with_bind_group(0, BindGroupHandle(1), vec![offset]);
}
```
