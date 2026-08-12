# Lõi ECS: Kiến Trúc Khung Xương (Framework Architecture)

Tài liệu này định nghĩa bản chất và cơ chế vận hành của hệ thống ECS trong `ifol-animation`. Tài liệu định nghĩa khung xương và các Render Component bắt buộc.

---

## 1. Bản Chất Của ECS

ECS là một cỗ máy xử lý dữ liệu theo pha (Phase Pipeline). Mục đích cuối cùng duy nhất của nó là: **Biến đổi dữ liệu phức tạp của Scene (phân cấp cha-con, tọa độ tương đối, thuộc tính vật lý,...) thành các danh sách Draw Call phẳng gửi cho GPU Engine vẽ.**

Bản thân ECS không quan tâm Component là gì. Nó chỉ quan tâm:
*   Các System có tạo ra hoặc biến đổi RenderNode cho Entity hay không.
*   Ở phase render cuối cùng, gom các RenderNode đó gửi xuống `ifol-gpu` vẽ 1 lần duy nhất.

---

## 2. Các Thành Phần Khung Xương

### 2.1. Entity
Chỉ là một ID định danh độc nhất (`EntityId`). Bản thân nó rỗng hoàn toàn.

### 2.2. Component
Các túi dữ liệu thuần túy (Pure Data Struct) được gắn vào Entity.

### 2.3. System
Các đơn vị logic xử lý dữ liệu theo pha (Phase).

### 2.4. World
Trình quản lý trung tâm chứa toàn bộ Entity, Components, và `RenderNodePool` Resource.

---

## 3. Các Render Component Bắt Buộc

Sự giao tiếp giữa đồ họa và hệ thống ECS thông qua các Component cốt lõi sau:

### 3.1. `DrawCacheComponent`
Là cái rương chứa kết quả tính toán đồ họa của một Entity.
*   **Chức năng:** Mỗi Entity có khả năng hiển thị (Shape, Image, Camera, SubGraph) lưu một con trỏ `node_id: RenderNodeId`.
*   **Arena Lookup:** Node thực sự nằm trong `RenderNodePool` (sống trong ECS World Resource).
*   **Tính chất:** Nhờ có con trỏ này, hệ thống tái sử dụng (Cache) `RenderBundle` của frame trước nếu Entity không có sự thay đổi.

### 3.2. `RenderRequestComponent`
Là điểm kích hoạt yêu cầu vẽ, gắn vào một Entity rỗng đại diện cho một Viewport/Màn hình Editor. Nó chứa:
*   `source_camera: EntityId` — Trỏ đến Entity Camera muốn xuất ra màn hình.
*   `output_target: RenderTarget` — Thông tin nơi GPU sẽ in kết quả ra (`Screen` hoặc `Offscreen`).

---

## 4. `RenderSystem` & Phong Bì `RenderGraph`

`RenderGraph` không phải là thứ do Camera hay Entity sở hữu cố định. Nó là một **"Phong bì tạm thời"** do `RenderSystem` tạo ra ở cuối mỗi frame:

```rust
fn render_system(world: &mut World, gpu_engine: &GpuEngine, executor: &RenderGraphExecutor) {
    // 1. Quét tìm tất cả RenderRequestComponent
    for (request_entity, request) in world.query::<&RenderRequestComponent>() {
        // 2. Lấy danh sách node_ids từ Camera được chỉ định
        let camera_cache = world.get::<DrawCacheComponent>(request.source_camera);
        let camera_node_ids = camera_cache.node_ids.clone();

        // 3. Tạo "phong bì" RenderGraph tạm thời
        let root_graph = RenderGraph {
            target: request.output_target.clone(),
            clear_color: Some([0.1, 0.1, 0.1, 1.0]),
            depth_stencil: camera_cache.depth_handle,
            node_ids: camera_node_ids,
        };

        // 4. Gửi phong bì + pool xuống GPU Engine
        let mut pool = world.get_resource_mut::<RenderNodePool>();
        executor.execute(gpu_engine, &registry, &mut pool, &root_graph);
    }
}
```

---

## 5. Tổng Kết Kiến Trúc

*   **RenderNodePool:** Nằm trong ECS World Resource, chứa tất cả `RenderNode`.
*   **DrawCacheComponent:** Gắn vào từng Entity, trỏ đến `RenderNodeId` trong Pool.
*   **RenderRequestComponent:** Quyết định Target (`Screen`/`Offscreen`).
*   **RenderSystem:** Bọc Nodes từ Camera + Target từ Request -> `RenderGraph` gửi `ifol-gpu`.
*   **`ifol-gpu`:** Hoàn toàn mù quáng, chỉ nhận `RenderGraph` + `RenderNodePool` và biên dịch ra GPU CommandBuffer.
