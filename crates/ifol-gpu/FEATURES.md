# ifol-gpu Feature Matrix

Tài liệu này theo dõi các tính năng hiện có của lõi GPU, được cập nhật liên tục bởi AI Agent theo **Feature Tracking Rule**. Các module bên ngoài có thể tham chiếu tài liệu này để biết lõi GPU đang có năng lực gì.

## Tính năng khởi tạo (Initialization)
- `[x]` **Headless Builder (`GpuEngineBuilder`)**: Hỗ trợ khởi tạo hệ thống GPU mù lòa (không cần Window). Cho phép yêu cầu Backend cụ thể (Vulkan/Metal/GL).
- `[x]` **Fallback Mechanism**: Tự động lùi về chuẩn đồ họa thấp hơn (ví dụ WebGL2 defaults) nếu cấu hình cao hơn không được phần cứng hỗ trợ.
- `[x]` **Hardware Capabilities Scanning (`GpuCapabilities`)**: Bọc lại `wgpu::Limits` thành cấu trúc an toàn. Lấy ra được các cực trị phần cứng như `max_texture_dimension_2d`, `max_bind_groups`, `max_uniform_buffer_binding_size`, `max_vertex_buffers`.

## Tính năng đồ họa (Render Graph & Resources) - Sắp tới
- `[ ]` Resource Handles (TextureHandle, PipelineHandle).
- `[ ]` Texture & Uniform Cache (LRU, Ring Buffer).
- `[ ]` Render Graph Execution (Node, SubGraph, Compiler).

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
