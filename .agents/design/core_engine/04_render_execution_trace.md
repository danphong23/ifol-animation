# Câu Chuyện Thực Thi: Trace 1 Frame Từ ECS Feature Tới GPU

Tài liệu này là kịch bản minh họa boundary end-to-end. Chi tiết compile,
resource lifetime và submit bên trong `ifol-gpu` phải theo tài liệu chính thức
tại `crates/ifol-gpu/docs/`; nội dung minh họa dưới đây không tạo thêm invariant
GPU nếu public API không bảo đảm.

---

## 1. Kịch Bản Khung Hình (Scenario)

*   **Viewport 1 (Màn hình chính):** Nhìn vào một bãi cỏ (10.000 chiếc lá instancing), một khung hình Video, và một Nhân vật được gom nhóm (**SubGraph** có Blur).
*   **Viewport 2 (Preview nhỏ):** Soi cận cảnh đúng Nhân vật đó.
*   **VRAM:** Đang đầy nắp. Một file ảnh `bullet.png` bị thiếu trên ổ cứng.

---

## 2. Bước 1: Render Core Đóng Gói (Bên ngoài GPU)

`feature-render-core`, đang chạy như các system đã đăng ký trong ECS, tìm thấy 2
`RenderRequestComponent`, thu thập render contribution và tạo graph. ECS kernel
không biết `RenderGraph` hoặc `ifol-gpu`:

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

## 3. Bước 2: Bắt Đầu Checked GPU Execution

Render Core gọi checked execution API. Cơ chế reset/reuse ring buffer, tracking
submission và frame-in-flight là implementation nội bộ của `ifol-gpu`, không do
ECS hoặc tài liệu workspace điều khiển.

---

## 4. Bước 3: Checked Execution Viewport 1

Graph của Viewport 1 có thể chứa draw, compute, copy và nested graph/subgraph.
`ifol-gpu` validate dependency/resource usage, flatten/compile và execute theo
public contract hiện hành. Nếu graph hoặc resource không hợp lệ, typed error được
trả về thay vì để Render Core tiếp tục im lặng.

Frame video hoặc missing-image fallback đã được feature/service resolve trước
execution. GPU graph chỉ thấy resource handle và command chuẩn hóa.

---

## 5. Bước 4: Checked Execution Viewport 2

Render Core có thể reuse render-owned contribution/cache khi target semantics,
resource revisions và GPU contract cho phép. Hai viewport không mặc nhiên dùng
chung bundle hoặc output nếu camera/target/bindings khác nhau. Correctness được
ưu tiên; reuse chỉ được bật khi dependency key chứng minh tương thích.

---

## 6. Bước 5: Trả Kết Quả Cho Host

`ifol-gpu` trả typed execution report/error/readback theo API được gọi. Không xem
"một submit cho mọi viewport" hoặc một mức FPS cụ thể là invariant cấp workspace.
Render Core chuyển kết quả thành `FrameResult`; `ifol-engine` quyết định present,
trả CLI output, tiếp tục export job hay broadcast event tới UI/MCP.
