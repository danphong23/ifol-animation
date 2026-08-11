# Thiết kế Cấu trúc Đồ thị Render (Render Graph) của `ifol-gpu`

## 1. Bản Chất Đệ Quy (Recursive) Của Render Graph

Trong các Engine thông thường, Render Graph là một danh sách phẳng. Tuy nhiên, với đặc thù của `ifol-animation` — một phần mềm Motion Graphics đòi hỏi tính năng **Pre-comp (Composition lồng Composition)** giống After Effects, Render Graph của chúng ta **bắt buộc phải có cấu trúc Đệ Quy (Cây lồng nhau)**.

Sự lồng nhau này giải quyết triệt để bài toán: **Xử lý hiệu ứng/độ mờ trên cả một Group**.
*Ví dụ:* Một nhân vật gồm Tay, Chân, Đầu chồng lên nhau. Nếu ta chỉnh Alpha = 50% cho từng bộ phận, phần giao nhau giữa Tay và Thân sẽ bị hiện rõ nét cắt (vì 2 lớp bán trong suốt đè lên nhau). 
Để giải quyết, ta gom nhân vật thành một `SubGraph`. Engine sẽ vẽ nhân vật đó với Alpha 100% vào một tấm ảnh nháp (Off-screen Texture), sau đó lấy tấm ảnh đó vẽ lên màn hình chính với Alpha 50%. Lúc này nhân vật trông như một thực thể trong suốt hoàn hảo.

## 2. Cấu Trúc Dữ Liệu: Node Lồng Node

Render Graph của `ifol-gpu` được thiết kế bằng Enum đệ quy để có thể chứa chính nó:

```rust
pub struct RenderGraph {
    pub nodes: Vec<RenderNode>,
}

pub enum RenderNode {
    /// Một lệnh vẽ đơn lẻ
    Draw(DrawCommand),
    
    /// Nhóm lồng nhau (Pre-comp). Engine sẽ tự xin cấp phát Off-screen Texture để vẽ nhóm này.
    SubGraph {
        name: String,
        graph: RenderGraph,                     // Đồ thị con nằm bên trong
        cache_id: String,                       // Tên Texture lưu kết quả vẽ
        post_pipeline: Option<PipelineHandle>,  // Shader áp dụng lên toàn nhóm (Ví dụ: Opacity 50%, Blur)
        is_dirty: bool,                         // Cờ kiểm soát Cache RenderBundle
        bundle: Option<wgpu::RenderBundle>      // Cache gói lệnh của GPU
    }
}

pub struct DrawCommand {
    pub mesh: MeshHandle,
    pub pipeline: PipelineHandle,
    pub bind_groups: Vec<BindGroupHandle>,
    // ... không chứa tọa độ, tọa độ nằm ở Uniform Buffer trên VRAM.
}
```

## 3. Kiến Trúc Stateful & RenderBundle Cache Trực Quan

`RenderGraph` đệ quy này sẽ được Engine giữ lại trong bộ nhớ (Retained-Mode).
Mỗi `SubGraph` đóng vai trò là một **`RenderPass`** độc lập và sở hữu một **`RenderBundle`** (Gói lệnh GPU) riêng.

### Kịch Bản Thay Đổi Dữ Liệu (The Update Scenario)

Hãy tưởng tượng Cây Render Graph như sau:
```text
Root Graph (Màn hình chính)
 ├── Draw(Background)
 └── SubGraph(Nhân Vật)
      ├── Draw(Tay)
      ├── Draw(Thân)
      └── Draw(Đầu)
```

**Tình huống 1: Di chuyển "Cái Tay"**
- ECS cập nhật tọa độ Tay vào Uniform Buffer trên VRAM.
- Không có Node nào bị thêm bớt. Không có cờ `is_dirty` nào bật lên.
- **Xử lý:** Engine lấy `RenderBundle` cũ của `SubGraph(Nhân Vật)` vứt cho GPU chạy. Thời gian CPU = 0ms. Tọa độ tự cập nhật.

**Tình huống 2: Đổi màu áo của "Thân" (Đổi Pipeline)**
- Lệnh vẽ `Draw(Thân)` bị thay đổi cấu trúc Pipeline.
- ECS gọi: `engine.mark_dirty("Nhân Vật")`.
- Cờ `is_dirty` của `SubGraph(Nhân Vật)` bật thành `true`. (Lưu ý: Root Graph không bị dirty).
- **Xử lý:** CPU mở `RenderBundleEncoder`, thu âm lại các lệnh của Tay, Thân, Đầu, lưu thành `RenderBundle` mới cho `SubGraph(Nhân Vật)`. CPU tốn ~0.5ms để xử lý riêng nhánh này.

**Tình huống 3: Áp Shader "Blur" cho Camera toàn cảnh**
- Bản chất Camera chứa Shader chính là một `SubGraph` bọc toàn bộ Scene.
- ECS cập nhật `post_pipeline` của Root. Tương tự, nếu cấu trúc cành nhánh con không đổi, chỉ có Root bị build lại Bundle.

## 4. Pipeline Thực Thi Trên GPU (Dữ Liệu Đẩy 1 Lần)

Khi `ifol-gpu` biên dịch (traverse) cái cây Render Graph này:
1. Nó duyệt đệ quy (hậu tố - postfix): Xử lý các `SubGraph` con trước. Xin VRAM tạo `RenderTarget` (Ảnh nháp).
2. Mở `RenderPass` cho từng `SubGraph`, đẩy `RenderBundle` vào.
3. Gom TẤT CẢ các Pass đó vào chung một `wgpu::CommandBuffer`.
4. Gọi hàm `queue.submit(...)` gửi **ĐÁNH GỤC ĐÚNG MỘT NHÁT** toàn bộ hệ thống đệ quy này sang cho GPU.
5. GPU tự động chạy các Pass từ dưới lên trên, tự động nối Texture con vào Texture cha và xuất ra màn hình.

*(Tài liệu này được cập nhật vào Phase 4.5)*
