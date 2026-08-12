# Quá Trình Phiên Dịch (Translation Pipeline)

Tài liệu này đặc tả cơ chế "Phiên dịch" — Cầu nối giữa hệ thống **ECS Logic (Cây Phân Cấp)** và **GPU Engine (`ifol-gpu`)**.

---

## 1. Bản Chất Quá Trình Phiên Dịch

Quá trình dịch từ Scene sang lệnh vẽ thực chất **là một chuỗi các System nằm trong ECS (`TranslationSystem`)**.

### Quy Tắc Phân Loại Node (Khi nào → DrawBatch, Khi nào → SubGraph?)

`TranslationSystem` khi duyệt qua một Entity sẽ áp dụng quy tắc đơn giản sau:

1. **Entity thông thường (Shape, Image, Text, 90% trường hợp):**
   - **Input Texture đã có sẵn** (nạp từ file `cat.png` hoặc màu sắc thuần).
   - 👉 Tạo **`DrawBatch` Node** trong `RenderNodePool` với 1 `DrawCommand` duy nhất.

2. **Entity nhóm / Composition có hiệu ứng không gian (Group + Blur, Drop Shadow):**
   - **Input Texture chưa tồn tại** — Cần phải vẽ toàn bộ các Entity con ra một tấm ảnh Offscreen trước, rồi mới lấy tấm ảnh đó làm Input cho hiệu ứng Blur.
   - 👉 Tạo **`SubGraph` Node** trong `RenderNodePool`.

---

## 2. Cơ Chế Tự Động Kết Nối Input Texture (Input Binding)

```text
[BƯỚC 1: TranslationSystem phát hiện Entity Group có Blur]
  - Tạo SubGraph_B với target = Offscreen(Texture_B).
  - Nhét các DrawBatch của Entity con vào inner_graph của SubGraph_B.

[BƯỚC 2: Tạo DrawCommand cho Node cha]
  - Shader `blur.wgsl` cần input texture ở `@binding(0)`.
  - ECS tự động lấy `TextureHandle(B)` gán vào `BindGroup` slot 0 của Node cha.

[BƯỚC 3: Thực thi trên GPU]
  - SubGraph_B vẽ xong toàn bộ con vào Texture_B.
  - Node cha chạy `blur.wgsl`, đọc `Texture_B` từ slot 0, nhả kết quả mờ ra màn hình.
```

---

## 3. Minh Họa Cấu Trúc Đệ Quy Đơn Giản

```text
Root RenderGraph (Target: Screen, từ RenderRequest)
│
├── Node 1 (DrawBatch): [DrawCommand: Vẽ Bầu Trời] (Texture: sky.png)
│
└── Node 2 (SubGraph "Project B"):
    ├── inner_graph (Target: Offscreen Texture_B 800x600):
    │   └── Node 2.1 (DrawBatch): [DrawCommand: Vẽ Cây]
    │   └── Node 2.2 (DrawBatch): [DrawCommand: Vẽ Cỏ]
    │
    └── commands: [
          DrawCommand(pipeline: blur_shader, bind: [Texture_B], action: Procedural(3))
        ]
        ↑ Lấy Texture_B, áp Blur, in lên Screen
```

---

## 4. Vai Trò Của `RenderNodePool` Trong Translation

Thay vì xây lại toàn bộ cây Node ở mỗi frame, `TranslationSystem` chỉ thao tác trên `RenderNodePool`:
*   Entity A chỉ di chuyển -> `TranslationSystem` cập nhật Ring Buffer VRAM. NodeId A giữ nguyên. `is_dirty` = `false`.
*   Entity B đổi Shader -> `TranslationSystem` gọi `pool.update_commands(node_id_b, new_cmds)`. NodeId B chuyển `is_dirty` = `true`.
*   Render System chỉ việc thu thập danh sách `node_ids` từ Camera và ném cho `ifol-gpu`.
