# 01. Kiến Trúc Render Graph & Lệnh Vẽ (Draw Command)

Tài liệu này định nghĩa cấu trúc dữ liệu cốt lõi của `ifol-gpu`. Mọi cấu trúc đều **ánh xạ 1:1 với API thật của wgpu** — không có hành vi ngầm định (No Magic).

---

## 1. Nguyên Tắc Thiết Kế

*   **Single Submission:** Mỗi khung hình, ECS dựng cấu trúc `RenderGraph`. `ifol-gpu` duyệt cây này, ghi MỌI LỆNH VẼ vào `wgpu::CommandEncoder`, rồi gửi xuống GPU (`queue.submit`) **MỘT LẦN DUY NHẤT**.
*   **GPU Mù Quáng:** `ifol-gpu` không biết ECS, Camera, Video, Layer là gì. Nó chỉ nhận cấu trúc dữ liệu và dịch thẳng ra lệnh wgpu.
*   **1 Graph = 1 RenderPass:** Tất cả các Node trong cùng một `RenderGraph` chia sẻ duy nhất một `RenderPass` trên GPU Target của Graph đó.
*   **ECS Toàn Quyền:** Mọi quyết định (Pipeline nào, Mesh gì, sắp xếp Đục/Mờ, kích thước Offscreen) đều do ECS tính toán trước.

---

## 2. Hệ Thống Định Danh (Handles & Node ID)

GPU Engine quản lý tài nguyên qua các Handle an toàn (số nguyên `u64`, O(1) Lookup). Tuyệt đối không dùng `String`.

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
pub struct RenderNodeId(pub u64);     // ID duy nhất của Node trong Arena Pool
```

---

## 3. Hành Động Vẽ (Draw Action)

Phần cứng GPU chỉ hỗ trợ đúng 2 kiểu quẹt cọ. Enum này ánh xạ 1:1 sang `wgpu::RenderPass::draw*`.

```rust
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawAction {
    /// Vẽ theo hình dáng Mesh có sẵn trong VRAM (Vertex + Index Buffer).
    /// → Ánh xạ: pass.set_vertex_buffer() + pass.set_index_buffer() + pass.draw_indexed()
    Indexed {
        mesh: MeshHandle,              // Lưới đỉnh nào
        index_range: Range<u32>,       // Phần nào của lưới (thường 0..tổng_index)
        instance_range: Range<u32>,    // Bao nhiêu bản sao (VD: 0..10000 cho Instancing)
    },

    /// Vẽ không cần Mesh — Shader tự tạo đỉnh từ vertex_index.
    /// → Ánh xạ: pass.draw(0..vertex_count, instance_range)
    /// Dùng khi: Fullscreen Post-FX Pass (Blur, Color Grading), Procedural Quad
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawCommand {
    /// Shader quyết định cách tô màu pixel.
    pub pipeline: PipelineHandle,

    /// Danh sách các túi dữ liệu (Uniform, Texture, Sampler).
    /// Mỗi phần tử = (khe_cắm, handle, dynamic_offsets_vào_ring_buffer)
    pub bind_groups: Vec<(u32, BindGroupHandle, Vec<u32>)>,

    /// Hành động quẹt cọ cụ thể (Indexed hoặc Procedural).
    pub action: DrawAction,
}
```

---

## 5. Đích Đến (Render Target)

"Bức tranh sẽ được in lên đâu?" — Do `RenderGraph` nắm giữ, không phải Node.

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderTarget {
    /// In thẳng ra cửa sổ hệ điều hành (Swap Chain)
    Screen,

    /// In ra một tấm ảnh ảo trong VRAM với kích thước chính xác.
    Offscreen {
        color: TextureHandle,       // Texture VRAM sẽ nhận kết quả vẽ
        width: u32,
        height: u32,
    },
}
```

---

## 6. Đồ Thị Vẽ (Render Graph) & Arena Pool

`RenderGraph` chứa danh sách các ID trỏ vào `RenderNodePool` (Arena Pattern).

```rust
#[derive(Debug, Clone)]
pub struct RenderGraph {
    pub target: RenderTarget,
    pub clear_color: Option<[f32; 4]>,
    pub depth_stencil: Option<TextureHandle>,
    pub node_ids: Vec<RenderNodeId>,
}
```

---

## 7. Nút Vẽ (Render Node)

Mỗi Node là "một hành động" trên bức tranh của Graph cha. Node sở hữu cache `bundle: Option<wgpu::RenderBundle>`.

```rust
#[derive(Debug, Clone)]
pub enum RenderNode {
    /// Nhóm đệ quy (Pre-comp / Group / Camera Post-FX).
    /// Quy trình: Vẽ graph con ra Offscreen trước, sau đó phát commands để in kết quả lên Graph cha.
    SubGraph {
        name: String,
        graph: Box<RenderGraph>,            // Đồ thị con vẽ ra Offscreen trước
        commands: Vec<DrawCommand>,         // Lệnh vẽ kết quả Offscreen lên Graph cha
        is_dirty: bool,                     // Cờ báo hiệu cần thu âm lại Bundle
        bundle: Option<wgpu::RenderBundle>, // Cache gói lệnh đã thu âm
    },

    /// Danh sách lệnh vẽ phẳng trên cùng 1 target.
    DrawBatch {
        commands: Vec<DrawCommand>,         // Danh sách lệnh vẽ (thường 1 command per entity)
        is_dirty: bool,                     // Cờ báo hiệu cần thu âm lại Bundle
        bundle: Option<wgpu::RenderBundle>, // Cache gói lệnh đã thu âm
    },
}
```

---

## 8. API Thao Tác Node (`RenderNodePool`)

Thư viện `ifol-gpu` cung cấp API quản lý Node trung tâm thông qua `RenderNodePool`:

```rust
impl RenderNodePool {
    pub fn alloc_batch(&mut self, commands: Vec<DrawCommand>) -> RenderNodeId;
    pub fn alloc_subgraph(&mut self, name: &str, graph: RenderGraph, commands: Vec<DrawCommand>) -> RenderNodeId;
    pub fn get(&self, id: RenderNodeId) -> Option<&RenderNode>;
    pub fn get_mut(&mut self, id: RenderNodeId) -> Option<&mut RenderNode>;
    pub fn update_commands(&mut self, id: RenderNodeId, commands: Vec<DrawCommand>) -> bool;
    pub fn mark_dirty(&mut self, id: RenderNodeId);
}
```

---

## 9. Thuật Toán Biên Dịch 2-Phase (Compiler)

`ifol-gpu` biên dịch cây `RenderGraph` thành chuỗi GPU CommandBuffer duy nhất:

```text
compile_graph(pool, graph):
    // PHASE 1: Bottom-up processing cho SubGraph
    for id in graph.node_ids:
        if pool[id] is SubGraph(inner_graph):
            compile_graph(pool, inner_graph) // Đệ quy: vẽ graph con ra Offscreen trước

    // PHASE 2: 1 RenderPass duy nhất cho Target hiện tại
    let pass = encoder.begin_render_pass(graph.target)
    for id in graph.node_ids:
        let node = pool[id]
        if node.is_dirty:
            node.bundle = record_bundle(node.commands)
            node.is_dirty = false
        pass.execute_bundles(&[node.bundle])
    pass.end_render_pass()
```

---

## 10. Cơ Chế Dirty & RenderBundle Cache

*   **Khi `is_dirty = true`:** `ifol-gpu` mở `RenderBundleEncoder`, thu âm lại lệnh vẽ trong Node, lưu binary vào `bundle`, set `is_dirty = false`.
*   **Khi `is_dirty = false`:** `ifol-gpu` lấy thẳng `bundle` cũ ném vào `pass.execute_bundles()`. Thời gian CPU $\approx 0$.
*   **Transform / Animation:** Thay đổi vị trí (Transform) KHÔNG làm dirty bundle vì vị trí nằm trong Uniform Ring Buffer (Dynamic Offset). Bundle chỉ chứa con trỏ chỉ vào Ring Buffer.
*   **Khi nào dirty?** Thêm/xóa DrawCommand, đổi Pipeline (Shader), đổi cấu trúc BindGroup.
