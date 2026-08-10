# ifol-gpu Feature Matrix

Tài liệu này theo dõi các tính năng hiện có của lõi GPU, được cập nhật liên tục bởi AI Agent theo **Feature Tracking Rule**. Các module bên ngoài có thể tham chiếu tài liệu này để biết lõi GPU đang có năng lực gì.

## Tính năng khởi tạo (Initialization)
- `[x]` **Headless Builder (`GpuEngineBuilder`)**: Hỗ trợ khởi tạo hệ thống GPU mù lòa (không cần Window). Cho phép yêu cầu Backend cụ thể (Vulkan/Metal/GL).
- `[x]` **Fallback Mechanism**: Tự động lùi về chuẩn đồ họa thấp hơn (ví dụ WebGL2 defaults) nếu cấu hình cao hơn không được phần cứng hỗ trợ.
- `[x]` **Hardware Capabilities Scanning (`GpuCapabilities`)**: Bọc lại `wgpu::Limits` thành cấu trúc an toàn. Lấy ra được các cực trị phần cứng như `max_texture_dimension_2d`, `max_bind_groups`, `max_uniform_buffer_binding_size`, `max_vertex_buffers`.

## Tính năng đồ họa (Render Graph & Resources) - Đã hoàn thành
- `[x]` **Resource Handles** (`TextureHandle`, `PipelineHandle`, `MeshHandle`): Đóng gói con trỏ tài nguyên thành chỉ số ID nguyên thủy (u64) nhằm tối ưu bộ nhớ và bảo đảm tính an toàn khi truyền qua Command Bus.
- `[x]` **Render Graph Data Structure**: Đồ thị có thứ tự chứa các `RenderNode`. Mỗi Node khai báo cấu hình đầu ra (`RenderTarget`) và chuỗi lệnh vẽ (`DrawCommand`).
- `[x]` **Render Graph Executor (Compiler)**: Bộ biên dịch đồ thị thành luồng lệnh phần cứng `wgpu::CommandEncoder`. Đi kèm `ResourceRegistry` để ánh xạ từ Handle siêu nhẹ ra các thực thể VRAM thực thụ.

## Quản lý Cửa sổ & Màn hình (Window & Surface) - Đã hoàn thành
- `[x]` **Surface Integration**: Hỗ trợ gắn `wgpu::Surface` kết hợp hoàn hảo với `winit 0.30` (theo mô hình `ApplicationHandler`) cho phép render trực tiếp ra cửa sổ hiển thị.
- `[x]` **Dynamic Surface Resizing**: Tự động cấu hình lại bộ đệm trình bày (Present buffer) của hệ điều hành thông qua hàm `resize_surface` và cơ chế lấy cấu hình tự động của chuẩn WGPU v30.

## Quản lý Bộ nhớ (Memory Management) - Đã hoàn thành
- `[x]` **Uniform Ring Buffer**: Cấp phát động dữ liệu Uniform với cơ chế quay vòng (Wrap-Around). Tự động tính toán Padding theo giới hạn căn lề chuẩn của phần cứng (`min_uniform_buffer_offset_alignment`).
- `[x]` **Texture Cache (Exact-Match Pooling)**: Tái sử dụng Texture qua các Frame bằng cơ chế Cache để tránh liên tục tạo/xóa VRAM, cực kỳ hữu dụng cho các đồ thị render phức tạp.

## Hướng dẫn sử dụng (Usage Examples)

### 1. Khởi tạo GPU Engine (Headless)

Ví dụ dưới đây là cách module bên ngoài (như `ifol-app-core`) gọi vào để khởi tạo GPU độc lập:

```rust
use ifol_gpu::api::GpuEngineBuilder;

async fn init_gpu() {
    // 1. Dùng Builder Pattern để khởi tạo Engine (tự động quyét Backend và Fallback)
    let engine = GpuEngineBuilder::new()
        .build()
        .await
        .expect("Lỗi: Không tìm thấy Card đồ họa tương thích!");

    // 2. Lấy thông số cực trị của phần cứng để chuẩn bị chiến lược Render
    let caps = engine.capabilities();
    println!("Max Texture Size hỗ trợ: {}", caps.max_texture_dimension_2d);
    
    // Nếu thiết bị yếu, có thể báo ngay cho UI/người dùng
    if caps.max_texture_dimension_2d < 4096 {
        log::warn!("Card đồ họa quá yếu để render 4K!");
    }
}
```

### 2. Xây dựng Render Graph

```rust
use ifol_gpu::render::{RenderGraph, RenderNode, RenderTarget, DrawCommand};
use ifol_gpu::render::{TextureHandle, MeshHandle, PipelineHandle, BindGroupHandle};

fn create_graph() {
    let mut graph = RenderGraph::new();

    // 1. Shadow Pass
    let shadow_target = RenderTarget {
        color_attachments: vec![],
        depth_attachment: Some(TextureHandle(1)), // Trỏ tới shadow map texture ID 1
    };
    let shadow_node = RenderNode::new("ShadowPass", shadow_target)
        .with_command(DrawCommand::DrawMesh {
            mesh: MeshHandle(100),
            pipeline: PipelineHandle(10),
            bind_groups: vec![BindGroupHandle(1)],
        });

    graph.add_node(shadow_node);
}
```

### 3. Cấp phát động Uniform Data (Ring Buffer)

```rust
use ifol_gpu::memory::UniformRingBuffer;
// ... (Khởi tạo GpuEngine) ...

fn render_frame(engine: &ifol_gpu::api::GpuEngine) {
    let alignment = engine.capabilities().min_uniform_buffer_offset_alignment;
    
    // Cấp phát 1 buffer 1MB cho toàn bộ object trong Frame
    let mut ring = UniformRingBuffer::new(engine.device(), 1024 * 1024, alignment);
    
    // Ghi dữ liệu ma trận (ví dụ: Matrix 4x4 ~ 64 bytes)
    let my_matrix_data = [1.0f32; 16]; 
    
    // Ghi vào Ring Buffer, Ring sẽ tự động căn lề (Padding) và trả về Dynamic Offset
    let offset = ring.write(engine.queue(), &my_matrix_data).unwrap();
    
    // --> Ném `offset` này vào DrawCommand để GPU dịch con trỏ đọc đúng vị trí
}
```

### 4. Biên dịch đồ thị và Thực thi (RenderGraphExecutor)

```rust
use ifol_gpu::render::{RenderGraphExecutor, ResourceRegistry};
// ... (Khởi tạo GpuEngine và RenderGraph) ...

fn render_frame(engine: &ifol_gpu::api::GpuEngine, graph: &ifol_gpu::render::RenderGraph) {
    let executor = RenderGraphExecutor::new();
    let registry = ResourceRegistry::new();
    
    // Đăng ký tài nguyên thật (VRAM) tương ứng với các ID trừu tượng trên đồ thị
    // registry.textures.insert(TextureHandle(1), texture_view);

    // Dịch đồ thị thành lệnh WGPU và đẩy xuống card màn hình
    let submission_idx = executor.execute(&engine, &registry, &graph);

    // Chờ GPU vẽ xong hoàn toàn (Thường dùng cho Integration Benchmark hoặc Readback)
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission_idx),
        timeout: None,
    });
}
```

### 5. Render lên cửa sổ (Visual Verification với Winit)

```rust
use ifol_gpu::api::GpuEngineBuilder;

// Bên trong vòng lặp EventLoop của winit 0.30 (ApplicationHandler)
fn render_to_window(engine: &ifol_gpu::api::GpuEngine, window: &winit::window::Window) {
    if let Some(surface) = engine.surface() {
        // Lấy frame hiện tại từ Surface
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return, // Bỏ qua nếu out-of-date, lost, timeout...
        };
        
        // ... (Build RenderGraph và Execute tương tự ví dụ 4) ...
        
        // Trình diễn Frame lên cửa sổ (wgpu v30 gọi từ Queue)
        engine.queue().present(frame);
    }
}
```
