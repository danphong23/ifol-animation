# 02. API Giao Tiếp, Cấu Hình & Kiểm Tra Thiết Bị

Lõi `ifol-gpu` được thiết kế dưới dạng một **Singleton Resource** độc lập hoàn toàn. Nó cung cấp các giao diện (API) rõ ràng để `ifol-ecs` (hoặc bất kỳ phần mềm nào khác) có thể khởi tạo, kiểm tra sức mạnh phần cứng, và cấu hình đồ họa.

---

## 1. Khởi Tạo & Singleton (Engine Builder)
Bên ngoài (ví dụ: `ifol-ecs`) sẽ không truy cập trực tiếp vào wgpu, mà thông qua một Builder an toàn.

```rust
// Khởi tạo Singleton
let gpu_engine = GpuEngineBuilder::new()
    .with_window(&window)            // Gắn vào cửa sổ Tauri/Winit
    .with_vsync(true)                // Bật/Tắt chống xé hình
    .with_msaa(4)                    // Khử răng cưa 4x
    .build()
    .await?;
```
Sau khi `build()`, `gpu_engine` sẽ được ECS đưa vào hệ thống dưới dạng `Global Resource` (Tài nguyên duy nhất), và tất cả các System khác có thể lấy ra để sử dụng (Ví dụ: System Tải Ảnh sẽ gọi `gpu_engine.create_texture(...)`).

---

## 2. Kiểm Tra Sức Mạnh Phần Cứng (Device Capabilities)
Mỗi máy tính (Card Màn Hình) có một sức mạnh khác nhau. `ifol-gpu` cung cấp API để phần mềm bên trên (ECS/UI) biết được giới hạn của máy tính đó là gì, từ đó tự động hạ cấu hình hoặc từ chối import file quá nặng.

```rust
pub struct GpuCapabilities {
    pub max_texture_dimension: u32,  // VD: 8192px (Không cho import ảnh to hơn mức này)
    pub max_bind_groups: u32,        // Giới hạn số lượng Uniforms
    pub supports_compute: bool,      // Máy có hỗ trợ Compute Shader không?
    pub supports_indirect: bool,     // Máy có hỗ trợ GPU-Driven Rendering không?
}

// Cách gọi:
let caps = gpu_engine.get_capabilities();
if !caps.supports_compute {
    println!("Máy tính quá cũ, sẽ tính toán Vật lý bằng CPU thay vì GPU!");
}
```
Việc lấy cấu hình này rất quan trọng khi chúng ta muốn nâng cấp lên Phase 2 (Compute Shader) như đã đề cập ở tài liệu trước. Nó đảm bảo Engine không bao giờ bị Crash trên các máy tính yếu.

---

## 3. Giao Tiếp Render & Đẩy Dữ Liệu Mỗi Khung Hình (Frame API)
Ở mỗi Frame (60 FPS), vòng đời giao tiếp giữa ECS và GPU sẽ diễn ra theo 3 bước thông qua API công khai:

```rust
// Bước 1: ECS đẩy dữ liệu Uniform (Singleton) lên VRAM trước
gpu_engine.update_global_uniforms(time, camera_matrix);

// Bước 2: ECS đẩy mảng Tọa độ (Entity Transforms / UVs) lên VRAM
gpu_engine.update_entity_uniforms(&array_of_10000_transforms);

// Bước 3: ECS đưa RenderGraph (hoặc ComputeGraph) cho GPU thực thi
gpu_engine.execute_render_graph(render_graph);
```

**Ranh giới tuyệt đối:** 
API của `ifol-gpu` chỉ nhận mảng byte `&[u8]` hoặc các Struct toán học cơ bản (`Mat4`, `Vec2`). Nó tuyệt đối không nhận các Object chứa Logic game (như `Entity`, `SpriteComponent`). Điều này giữ cho `ifol-gpu` mãi mãi "ngu ngốc" và tái sử dụng được ở bất cứ đâu!
