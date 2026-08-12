# Câu Chuyện Thực Thi: Trace 1 Frame Đồ Họa Cấp Thấp

Tài liệu này mô tả chi tiết từng bước (Trace) những gì xảy ra bên trong `ifol-gpu` khi nhận yêu cầu render 1 Frame với quy trình 2-Phase Compiler chuẩn.

---

## 1. Kịch Bản Khung Hình (Scenario)

*   **Viewport 1 (Màn hình chính):** Nhìn vào một bãi cỏ (10.000 chiếc lá instancing), một khung hình Video, và một Nhân vật được gom nhóm (**SubGraph** có Blur).
*   **Viewport 2 (Preview nhỏ):** Soi cận cảnh đúng Nhân vật đó.
*   **VRAM:** Đang đầy nắp. Một file ảnh `bullet.png` bị thiếu trên ổ cứng.

---

## 2. Bước 1: ECS Đóng Gói (Bên ngoài GPU)

ECS `RenderSystem` tìm thấy 2 `RenderRequestComponent` đại diện 2 Viewport, tạo 2 phong bì `RenderGraph` gửi xuống `ifol-gpu`:

```json
[
  // Viewport 1
  RootGraph_Viewport_1 {
    target: Screen,
    node_ids: [Node_SubGraph_Char, Node_Batch_Grass, Node_Batch_Video, Node_Batch_Bullet]
  },
  // Viewport 2
  RootGraph_Viewport_2 {
    target: Offscreen_Preview,
    node_ids: [Node_SubGraph_Char] // Dùng chung Node_SubGraph_Char từ Pool!
  }
]
```

---

## 3. Bước 2: Đầu Frame (Reset Ring Buffer)

GPU Engine kéo con trỏ `UniformRingBuffer` về `0` (0 cost allocation). Mọi dữ liệu Uniforms của frame cũ bị lờ đi, sẵn sàng bị chép đè.

---

## 4. Bước 3: Biên Dịch Viewport 1 (2-Phase Compiler)

### PHASE 1: Đệ Quy Xử Lý SubGraph (Bottom-Up)
*   Compiler duyệt `node_ids` của Viewport 1, phát hiện `Node_SubGraph_Char`.
*   Compiler **chưa mở RenderPass cho Screen**. Nó đi xin VRAM 1 tấm ảnh `Texture_Char` (600x800).
*   Nó đệ quy compile `graph` con bên trong SubGraph, mở RenderPass cho `Texture_Char`, vẽ Tay, Chân, Đầu vào đó.
*   Vẽ xong, `Texture_Char` có chứa ảnh nhân vật hoàn chỉnh. **Phase 1 kết thúc.**

### PHASE 2: Mở 1 GPU RenderPass Duy Nhất Cho Screen
Compiler gọi `encoder.begin_render_pass(Screen)`. Tất cả các Node dưới đây chia sẻ **cùng 1 Pass này**:

1.  **Thực thi `Node_SubGraph_Char`:** Node này có `is_dirty = false` -> Compiler phát `bundle` cũ. Lệnh trong bundle nạp `Texture_Char` vừa vẽ ở Phase 1, chạy `blur.wgsl`, vẽ nhân vật mờ lên Screen.
2.  **Thực thi `Node_Batch_Grass` (10.000 chiếc lá):** 
    - `is_dirty = false` -> phát `bundle` cũ.
    - Lệnh vẽ instancing 10.000 bản sao trong 1 nhịp chớp mắt.
3.  **Thực thi `Node_Batch_Video`:** 
    - `ffmpeg` nhả byte video mới. VRAM đầy OOM -> **LRU Cache** kích hoạt đá `logo.png` cũ ra khỏi VRAM.
    - `write_texture` chép đè byte video vào vùng trống. Phát `bundle` video.
4.  **Thực thi `Node_Batch_Bullet` (Missing file):** 
    - ECS đã gán Texture caro mặc định vào `TextureHandle(12)`.
    - Phát `bundle` viên đạn caro hồng/đen mờ.

Compiler gọi `pass.end_render_pass()`. Viewport 1 hoàn tất!

---

## 5. Bước 4: Biên Dịch Viewport 2 (Tái Sử Dụng Bundle)

Compiler chuyển sang `RootGraph_Viewport_2`:
*   Viewport 2 chiếu cận cảnh nhân vật, dùng chung `Node_SubGraph_Char` từ `RenderNodePool`.
*   `Node_SubGraph_Char` đã có sẵn `Texture_Char` vẽ từ Phase 1 và `bundle` đã thu âm.
*   Compiler mở RenderPass cho `Offscreen_Preview`, phát lại `bundle` đó với ma trận phóng to.
*   **Kết quả:** Dù nhân vật có 100 cái xương, GPU **không tốn 1 giọt mồ hôi** tính lại cấu trúc. Nhanh cực đại!

---

## 6. Bước 5: Chốt Frame (Submit 1 Lần)

Sau khi duyệt hết 2 RootGraph:
*   GPU Engine gọi `queue.submit(encoder.finish())` — **Ném 1 CommandBuffer duy nhất xuống GPU**.
*   GPU thực thi liên hoàn tất cả RenderPass. Mượt mà 144 FPS!
