# Thiết Kế Cấu Trúc Đồ Thị Render (Render Graph) Của `ifol-gpu`

> **Lưu ý:** Tài liệu này là bản tổng quan. Chi tiết cấu trúc dữ liệu chính thức nằm tại:
> 👉 [gpu_engine/render/01_render_graph_and_command.md](core_engine/gpu_engine/render/01_render_graph_and_command.md)

## 1. Bản Chất Đệ Quy (Recursive) Của Render Graph

Trong các Engine thông thường, Render Graph là một danh sách phẳng. Tuy nhiên, với đặc thù của `ifol-animation` — một phần mềm Motion Graphics đòi hỏi tính năng **Pre-comp (Composition lồng Composition)** giống After Effects, Render Graph của chúng ta **bắt buộc phải có cấu trúc Đệ Quy (Cây lồng nhau)**.

Sự lồng nhau này giải quyết triệt để bài toán: **Xử lý hiệu ứng/độ mờ trên cả một Group**.
*Ví dụ:* Một nhân vật gồm Tay, Chân, Đầu chồng lên nhau. Nếu ta chỉnh Alpha = 50% cho từng bộ phận, phần giao nhau giữa Tay và Thân sẽ bị hiện rõ nét cắt (vì 2 lớp bán trong suốt đè lên nhau).
Để giải quyết, ta gom nhân vật thành một `SubGraph`. Engine sẽ vẽ nhân vật đó với Alpha 100% vào một tấm ảnh nháp (Off-screen Texture), sau đó lấy tấm ảnh đó vẽ lên màn hình chính với Alpha 50%. Lúc này nhân vật trông như một thực thể trong suốt hoàn hảo.

## 2. Cấu Trúc Dữ Liệu: Node Lồng Node

`RenderGraph` chứa danh sách `RenderNode`. Mỗi Node có thể là:
*   **`DrawBatch`:** Danh sách lệnh `DrawCommand` vẽ phẳng trên target của Graph.
*   **`SubGraph`:** Chứa một Graph con (vẽ ra Offscreen trước) + danh sách lệnh `DrawCommand` để vẽ kết quả Offscreen lên target của Graph cha.

**Quy tắc "No Magic":** SubGraph không tự động biết cách in ảnh lên cha. ECS phải nhét sẵn Pipeline, BindGroup (chứa Texture Offscreen), và DrawAction vào danh sách `commands` của SubGraph. `ifol-gpu` chỉ nhắm mắt thực thi.

*(Chi tiết cấu trúc Rust: xem tài liệu `01_render_graph_and_command.md`)*

## 3. Kiến Trúc Stateful & RenderBundle Cache Trực Quan

`RenderGraph` đệ quy này sẽ được Engine giữ lại trong bộ nhớ (Retained-Mode).
Mỗi `RenderNode` sở hữu một **`RenderBundle`** (Gói lệnh GPU đã thu âm) riêng.

### Kịch Bản Thay Đổi Dữ Liệu

Hãy tưởng tượng Cây Render Graph như sau:
```text
Root Graph (Màn hình chính)
 ├── DrawBatch(Background)
 └── SubGraph(Nhân Vật)
      ├── graph con: DrawBatch(Tay, Thân, Đầu) → vẽ ra Offscreen
      └── commands: [DrawCommand(áp Shader lên Offscreen, in lên Screen)]
```

**Tình huống 1: Di chuyển "Cái Tay"**
- ECS cập nhật tọa độ Tay vào Uniform Ring Buffer trên VRAM.
- Không có Node nào bị thêm bớt. Cờ `is_dirty` vẫn = `false`.
- **Xử lý:** Engine lấy `RenderBundle` cũ chạy. Thời gian CPU = 0ms.

**Tình huống 2: Đổi Shader của "Thân" (Đổi Pipeline)**
- Lệnh vẽ `Draw(Thân)` bị thay đổi Pipeline.
- Cờ `is_dirty` của Node chứa nó bật thành `true`.
- **Xử lý:** CPU mở `RenderBundleEncoder`, thu âm lại lệnh. Chỉ Node bị bẩn mới tốn chi phí.

## 4. Pipeline Thực Thi Trên GPU (Dữ Liệu Đẩy 1 Lần)

Khi `ifol-gpu` biên dịch cái cây Render Graph này:
1. Nó duyệt đệ quy theo chiều sâu (Depth-First): Xử lý các SubGraph con trước.
2. Mỗi SubGraph: Xin VRAM tạo Offscreen → Vẽ graph con vào đó → Thực thi `commands` lên target cha.
3. Gom TẤT CẢ các RenderPass đó vào chung một `wgpu::CommandBuffer`.
4. Gọi hàm `queue.submit(...)` gửi **MỘT LẦN DUY NHẤT** toàn bộ xuống GPU.
