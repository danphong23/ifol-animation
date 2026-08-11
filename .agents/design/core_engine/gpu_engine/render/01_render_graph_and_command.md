# 01. Kiến Trúc Render Graph & Lệnh Vẽ (Draw Command)

Tài liệu này định nghĩa cấu trúc dữ liệu cốt lõi của `ifol-gpu`. Mọi cấu trúc đều **ánh xạ 1:1 với API thật của wgpu** — không có hành vi ngầm định (No Magic).

---

## 1. Nguyên Tắc Thiết Kế

*   **Single Submission:** Mỗi khung hình, ECS xây toàn bộ đồ thị `RenderGraph` (cây đệ quy). `ifol-gpu` duyệt cây này, ghi MỌI LỆNH VẼ vào `wgpu::CommandEncoder`, rồi gửi xuống GPU (`queue.submit`) **MỘT LẦN DUY NHẤT**.
*   **GPU Mù Quáng:** `ifol-gpu` không biết ECS, Camera, Video, Layer là gì. Nó chỉ nhận cấu trúc dữ liệu và dịch thẳng ra lệnh wgpu.
*   **ECS Toàn Quyền:** Mọi quyết định (Pipeline nào, Mesh gì, sắp xếp Đục/Mờ, kích thước Offscreen, Padding Blur) đều do ECS tính toán trước.

---

## 2. Hệ Thống Định Danh (Handles)

GPU Engine quản lý tài nguyên qua các Handle an toàn (số nguyên, O(1) Lookup). Tuyệt đối không dùng `String`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineHandle(pub u64);   // ID của Shader Code đã biên dịch

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u64);    // ID của Ảnh/Video/Offscreen trên VRAM

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BindGroupHandle(pub u64);  // ID của túi dữ liệu (Uniform + Texture + Sampler)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshHandle(pub u64);       // ID của lưới đỉnh (Vertex + Index Buffer) trên VRAM

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle(pub u64);     // [Tương lai] ID của Storage Buffer cho Compute Shader
```

---

## 3. Hành Động Vẽ (Draw Action)

Phần cứng GPU chỉ hỗ trợ đúng 2 kiểu quẹt cọ. Enum này ánh xạ 1:1 sang `wgpu::RenderPass::draw*`.

```rust
use std::ops::Range;

pub enum DrawAction {
    /// Vẽ theo hình dáng Mesh có sẵn trong VRAM (Vertex + Index Buffer).
    /// → Ánh xạ: pass.set_vertex_buffer() + pass.set_index_buffer() + pass.draw_indexed()
    ///
    /// Dùng khi: Vẽ ảnh lên Quad, vẽ nhân vật 3D, vẽ 10.000 cái lá (Instancing)
    Indexed {
        mesh: MeshHandle,              // Lưới đỉnh nào
        index_range: Range<u32>,       // Phần nào của lưới (thường 0..tổng_index)
        instance_range: Range<u32>,    // Bao nhiêu bản sao (VD: 0..10000 cho Instancing)
    },

    /// Vẽ không cần Mesh — Shader tự tạo đỉnh từ vertex_index.
    /// → Ánh xạ: pass.draw(0..vertex_count, instance_range)
    ///
    /// Dùng khi: Full-screen Shader Pass (Blur, Color Grading), UI Quad
    /// vertex_count = 3 → Shader tạo 1 tam giác khổng lồ bao trọn toàn màn hình
    Procedural {
        vertex_count: u32,             // Số đỉnh ảo Shader tự tạo (thường = 3)
        instance_range: Range<u32>,    // Bao nhiêu bản sao
    },
}
```

---

## 4. Lệnh Vẽ Hoàn Chỉnh (Draw Command)

Mỗi lệnh = 1 lần quẹt cọ trên bức tranh. Cấu trúc ánh xạ 1:1 với chuỗi gọi hàm wgpu.

```rust
pub struct DrawCommand {
    /// Bắt buộc: Shader quyết định cách tô màu pixel.
    /// → Ánh xạ: pass.set_pipeline(&pipeline)
    pub pipeline: PipelineHandle,

    /// Bắt buộc: Danh sách các túi dữ liệu (Uniform, Texture, Sampler).
    /// Mỗi phần tử = (khe_cắm, handle, offset_vào_ring_buffer)
    /// → Ánh xạ: pass.set_bind_group(index, &bg, &offsets)
    ///
    /// BindGroup chứa cả Uniform (số liệu) lẫn Texture (ảnh nguồn).
    /// dynamic_offsets trỏ đúng vị trí trong Uniform Ring Buffer cho từng Entity.
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,

    /// Bắt buộc: Hành động quẹt cọ cụ thể (Indexed hoặc Procedural).
    pub action: DrawAction,
}
```

**Tại sao cấu trúc này sạch:**
*   `pipeline` luôn bắt buộc — DrawCommand nào cũng phải có Shader.
*   `bind_groups` gom chặt `index + handle + offsets` — offset không trôi nổi, gắn đúng vào BindGroup tương ứng.
*   `action` là Enum rạch ròi — muốn Mesh hay Procedural phải chọn rõ, không có `mesh: Option<>` mập mờ.

---

## 5. Đích Đến (Render Target)

"Bức tranh sẽ được in lên đâu?" — Do `RenderGraph` nắm giữ, không phải Node.

```rust
pub enum RenderTarget {
    /// In thẳng ra cửa sổ hệ điều hành (Swap Chain)
    Screen,

    /// In ra một tấm ảnh ảo trong VRAM với kích thước chính xác.
    /// ECS quyết định kích thước (bao gồm Padding mở rộng cho Blur/Glow).
    Offscreen {
        color: TextureHandle,       // Texture VRAM sẽ nhận kết quả vẽ
        width: u32,
        height: u32,
    },
}
```

---

## 6. Đồ Thị Vẽ (Render Graph)

Một "tấm toan" (Canvas) chứa danh sách các nét cọ. Graph có thể chứa Graph con (Đệ quy vô hạn).

```rust
pub struct RenderGraph {
    /// Bức tranh này sẽ được in ra đâu?
    pub target: RenderTarget,

    /// Xóa phông nền trước khi vẽ (None = vẽ đè lên nội dung cũ)
    pub clear_color: Option<[f32; 4]>,

    /// [Sẵn sàng 3D] Depth/Stencil Texture dùng chung cho toàn bộ Graph này.
    /// Vật Đục ghi depth, Vật Mờ chỉ đọc không ghi. Cùng chia sẻ 1 tấm Z-Buffer.
    /// None = chế độ 2D thuần (không dùng Z-Buffer).
    pub depth_stencil: Option<TextureHandle>,

    /// Danh sách các nút vẽ. Thứ tự 0 → N = thứ tự vẽ đè lên nhau.
    /// ECS chịu trách nhiệm sắp xếp (Đục gần→xa trước, Mờ xa→gần sau).
    pub nodes: Vec<RenderNode>,
}
```

---

## 7. Nút Vẽ (Render Node)

Mỗi Node là "một hành động" trên bức tranh của Graph cha. Node có 2 dạng: **Vẽ phẳng (DrawBatch)** hoặc **Đệ quy lồng nhau (SubGraph)**.

**Đặc điểm quan trọng:** Cả 2 dạng đều có `commands: Vec<DrawCommand>` — SubGraph chỉ khác ở chỗ nó có thêm một Graph con được vẽ trước.

```rust
pub enum RenderNode {
    /// Nhóm đệ quy (Pre-comp / Group / Camera Post-FX).
    ///
    /// Quy trình thực thi của ifol-gpu:
    /// 1. Nhảy vào `graph` con, vẽ nó ra Offscreen (target của graph con).
    /// 2. Quay lại Graph cha, thực thi danh sách `commands` lên target của cha.
    ///    (ECS đã nhét TextureHandle của Offscreen vào BindGroup trong commands)
    ///
    /// Ví dụ: Graph con vẽ nhân vật ra Offscreen. Commands lấy Offscreen đó,
    /// áp Shader Blur, rồi in lên màn hình chính.
    ///
    /// Nếu `commands` rỗng → Graph con chỉ vẽ ra Offscreen mà không in lên cha
    /// (dùng cho Shadow Map, G-Buffer — chỉ tạo Texture dữ liệu).
    SubGraph {
        name: String,
        graph: Box<RenderGraph>,            // Đồ thị con vẽ ra Offscreen trước
        commands: Vec<DrawCommand>,         // Danh sách lệnh vẽ kết quả lên Graph cha
        is_dirty: bool,                     // Cờ báo hiệu cần Record lại Bundle
        bundle: Option<wgpu::RenderBundle>, // [Tương lai] Cache gói lệnh đã thu âm
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    /// ECS đã sắp xếp sẵn: cùng Pipeline gom lại, Đục/Mờ tách biệt.
    DrawBatch {
        commands: Vec<DrawCommand>,         // Danh sách lệnh vẽ
        is_dirty: bool,                     // Cờ báo hiệu cần Record lại Bundle
        bundle: Option<wgpu::RenderBundle>, // [Tương lai] Cache gói lệnh đã thu âm
    },
}
```

---

## 8. Cơ Chế Đệ Quy & RenderBundle Cache

### 8.1. Quy Trình Biên Dịch (Graph Compiler)

`ifol-gpu` duyệt cây đệ quy theo chiều sâu (Depth-First), đập phẳng thành chuỗi `RenderPass` tuyến tính:

```text
compile_graph(graph):
    for node in graph.nodes:
        match node:
            SubGraph { graph: inner, commands, .. }:
                ① compile_graph(inner)           // Đệ quy: vẽ graph con ra Offscreen
                ② begin_render_pass(graph.target) // Mở phiên vẽ trên target CỦA CHA
                   for cmd in commands:           // Vẽ kết quả Offscreen lên cha
                       execute(cmd)
                   end_render_pass()

            DrawBatch { commands, .. }:
                ① begin_render_pass(graph.target) // Mở phiên vẽ trên target của graph
                   for cmd in commands:
                       execute(cmd)
                   end_render_pass()

    → Kết quả cuối: 1 wgpu::CommandBuffer → queue.submit() MỘT LẦN
```

### 8.2. RenderBundle Cache (Triển Khai Tương Lai)

*   **Khi `is_dirty = true`:** `ifol-gpu` mở `RenderBundleEncoder`, thu âm toàn bộ lệnh vẽ bên trong Node đó (hoặc đệ quy thu âm SubGraph), lưu binary vào `bundle`, rồi set `is_dirty = false`.
*   **Khi `is_dirty = false`:** `ifol-gpu` KHÔNG duyệt vào `commands` nữa. Nó lấy thẳng `bundle` cũ ném vào `CommandEncoder`. Thời gian CPU gần bằng 0.
*   **Tính Độc Lập:** Nếu đổi màu áo nhân vật trong `SubGraph(Nhân Vật)`, chỉ SubGraph đó bị bật `is_dirty`. `SubGraph(Môi Trường)` và Graph cha vẫn xài Bundle cũ.

### 8.3. Khi Nào `is_dirty = true`?

*   Thêm / Xóa DrawCommand trong Node.
*   Đổi Pipeline (Shader) của DrawCommand.
*   Đổi Mesh (Hình dáng) của DrawCommand.
*   Đổi BindGroup Handle (Nguồn dữ liệu thay đổi cấu trúc).

### 8.4. Khi Nào KHÔNG Dirty?

*   **Thay đổi Transform (Vị trí, Xoay, Phóng to):** Vì Transform nằm trong Uniform Ring Buffer. Bundle chỉ thu âm "con trỏ" chỉ đến Ring Buffer, không thu âm giá trị. Khi vật thể di chuyển, giá trị trong Ring Buffer thay đổi, nhưng Bundle vẫn đúng vì nó vẫn trỏ đúng chỗ.
*   **Cập nhật nội dung Texture (Video frame mới):** Texture Handle không đổi, chỉ có pixel bên trong VRAM thay đổi (Fast-Update / write_texture).

---

## 9. Ví Dụ Trực Quan: Scene 3D Phức Tạp

Kịch bản: **Nhân vật (Group có Blur) đứng trước Ngôi nhà 3D, có Bóng đổ, có UI đè lên.**

```text
RootGraph (Target: Screen, Depth: depth_main)
│
├── Node 0: SubGraph "Shadow Map"
│   ├── graph.target = Offscreen(shadow_tex, 2048x2048)
│   ├── graph.depth_stencil = Some(shadow_depth)
│   ├── graph.nodes = [DrawBatch(vẽ scene từ góc nhìn Đèn)]
│   └── commands = []  ← RỖNG: Chỉ tạo Shadow Map, không in lên Screen
│
├── Node 1: SubGraph "Nhân Vật (Group Blur)"
│   ├── graph.target = Offscreen(char_tex, 600x800)
│   ├── graph.nodes = [DrawBatch(vẽ Tay, Chân, Đầu vào char_tex)]
│   └── commands = [
│         DrawCommand(pipe: blur, bind: [char_tex + blur_params], action: Procedural(3))
│       ]
│       ↑ Lấy char_tex, áp Blur, vẽ kết quả lên Screen (target của RootGraph)
│
├── Node 2: DrawBatch "Vật Đục (Opaque)"
│   commands = [
│     DrawCommand(pipe: pbr, bind: [shadow_tex, house_mat], action: Indexed(house_mesh))
│     DrawCommand(pipe: pbr, bind: [shadow_tex, tree_mat], action: Indexed(tree_mesh, ..., 0..50))
│   ]
│   // ECS đã sắp: Gần→Xa, Pipeline giống gom lại. DepthWrite = ON.
│
├── Node 3: DrawBatch "Vật Mờ (Transparent)"
│   commands = [
│     DrawCommand(pipe: pbr_transparent, bind: [glass_mat], action: Indexed(window_mesh))
│   ]
│   // ECS đã sắp: Xa→Gần. DepthTest = ON, DepthWrite = OFF.
│
└── Node 4: DrawBatch "UI Layer"
    commands = [
      DrawCommand(pipe: ui, bind: [button_tex], action: Procedural(3))
    ]
    // DepthTest = OFF. UI luôn vẽ cuối, đè lên tất cả.
```

**Thứ tự GPU thực thi (đã đập phẳng):**
1. `RenderPass 1` → Target: `shadow_tex` → Vẽ scene từ góc Đèn
2. `RenderPass 2` → Target: `char_tex` → Vẽ Tay, Chân, Đầu
3. `RenderPass 3` → Target: `Screen` → In nhân vật (đã Blur) lên Screen
4. `RenderPass 4` → Target: `Screen` → Vẽ Nhà + 50 Cây (Opaque)
5. `RenderPass 5` → Target: `Screen` → Vẽ Cửa Kính (Transparent)
6. `RenderPass 6` → Target: `Screen` → Vẽ UI
7. `queue.submit()` → **GỬI 1 LẦN DUY NHẤT**
