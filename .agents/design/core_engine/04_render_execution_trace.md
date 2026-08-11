# Câu Chuyện Thực Thi: Trace 1 Frame Đồ Họa Cấp Thấp

Để thực sự hiểu cách hệ thống hoạt động, tài liệu này mô tả chi tiết từng mili-giây (Trace) những gì xảy ra bên trong `ifol-gpu` khi nó nhận yêu cầu render 1 Frame cực kỳ phức tạp.

## 1. Kịch Bản Khung Hình (The Scenario)

Hãy tưởng tượng bạn đang mở phần mềm ifol-animation, khung hình hiện tại có cấu hình như sau:
*   **Viewport 1 (Màn hình chính):** Đang nhìn vào một bãi cỏ có **10.000 chiếc lá**, một **màn hình TV đang phát Video**, và một nhân vật được gom nhóm (**Pre-comp**).
*   **Viewport 2 (Màn hình Preview nhỏ):** Đang soi cận cảnh vào đúng cái nhân vật Pre-comp đó.
*   **Tình trạng phần cứng:** VRAM (RAM của card đồ họa) đang bị đầy tới nắp. Có một file ảnh `bullet.png` bị xóa mất ngoài ổ cứng (thiếu file).

---

## 2. Bước 1: ECS Nén Dữ Liệu (Bên ngoài Engine)

Trước khi GPU Engine bắt tay vào việc, hệ thống ECS (Tài liệu 01 & 03) đã chạy xong Phase cuối cùng và tổng hợp ra một mảng gồm 2 Render Graph đưa cho GPU:

```json
[
  // Đích đến: Cửa sổ Viewport 1
  RootGraph_Viewport_1: [
      SubGraph_Character (Pre-comp), 
      DrawBatch [DrawCommand (10.000 lá cỏ - ĐÃ ĐƯỢC GỘP)], 
      DrawBatch [DrawCommand (Khung hình Video)],
      DrawBatch [DrawCommand (Viên đạn - bị thiếu ảnh)]
  ],
  // Đích đến: Cửa sổ Viewport 2
  RootGraph_Viewport_2: [
      DrawBatch [DrawCommand (Chỉ vẽ lại kết quả của SubGraph_Character)]
  ]
]
```

👉 **GPU Engine nhận mảng dữ liệu này.** Từ thời điểm này trở đi, GPU Engine nhắm mắt làm theo mệnh lệnh.

---

## 3. Bước 2: Dọn dẹp & Khởi động (Đầu Frame)

1.  **Reset Ring Buffer:** GPU Engine nắm giữ một cục RAM siêu to (Ví dụ 5MB) chuyên dùng chứa Uniforms. Nó cầm con trỏ (pointer) kéo bùm một phát về vị trí `0`. **(0 cost allocation)**. Toàn bộ rác của frame trước bị lờ đi, sẵn sàng bị ghi đè.

---

## 4. Bước 3: Thực thi Viewport 1

Engine duyệt mảng, bắt đầu với `RootGraph_Viewport_1`.

### Hành động 3.1: Gặp SubGraph_Character (Đệ quy)
*   Engine thấy đây không phải lệnh vẽ trực tiếp mà là một Sub-Graph. Nó **tạm dừng** mạch vẽ chính.
*   Nó đi xin VRAM một tấm ảnh trống (Render Target) theo kích thước ECS yêu cầu.
*   Nó đi sâu vào trong SubGraph_Character, thực thi các DrawBatch bên trong đó (vẽ tay, chân, mặt) và in tất cả lên tấm ảnh trống vừa xin.
*   Vẽ xong, nó gán ID cho tấm ảnh đó, gọi là `TextureHandle(5)`. Quay lại mạch vẽ chính và thực thi danh sách `commands` của SubGraph này để in `TextureHandle(5)` lên màn hình.

### Hành động 3.2: Gặp DrawBatch (10.000 chiếc lá cỏ)
*   **Batching bẩm sinh:** ECS đã cực kỳ thông minh, thay vì đưa 10.000 lệnh, nó chỉ đưa 1 lệnh duy nhất. Mảng Uniforms đính kèm trong lệnh này chứa một chuỗi dài dằng dặc 10.000 tọa độ.
*   Engine copy chuỗi tọa độ này chép ụp vào Ring Buffer (Con trỏ Ring Buffer nhích lên 1 đoạn).
*   Engine gọi `pipeline = PipelineHandle(1)`, nạp `BindGroup` chứa ảnh cỏ.
*   Engine đọc `action = DrawAction::Indexed { instance_range: 0..10000 }`.
*   GPU giật điện, vẽ 10.000 chiếc lá trong 1 nhịp chớp mắt. Cực mượt.

### Hành động 3.3: Gặp DrawBatch (Khung hình Video)
*   Engine đọc lệnh thấy yêu cầu dùng `TextureHandle(9)`.
*   Phần mềm giải mã video (ffmpeg) vừa nhả ra một khung hình mới (mảng byte mới).
*   **Xử lý Edge Case (OOM):** Engine định nạp mảng byte này lên VRAM, nhưng phát hiện VRAM ĐÃ ĐẦY! Nếu cố nhét, app sẽ Crash.
*   **LRU Cache kích hoạt:** Engine lục trong từ điển, phát hiện tấm ảnh `"logo_cong_ty.png"` đã 10 phút rồi không xuất hiện trên màn hình. Nó lạnh lùng **đá (Evict)** tấm ảnh đó ra khỏi VRAM.
*   **Fast-Update:** Thay vì xin cấp phát VRAM mới (rất chậm), Engine tái sử dụng khoảng trống vừa được giải phóng, ghi đè (write_texture) byte video lên đó. Video chạy mượt 60fps.

### Hành động 3.4: Gặp DrawBatch (Viên đạn)
*   Engine đọc lệnh yêu cầu nạp `TextureHandle(12)`.
*   **Xử lý Edge Case (Missing File):** ECS không tìm thấy file trên ổ cứng khi nạp. Nhưng thay vì để trống, ECS đã gắn một Texture mặc định (caro) vào Handle này.
*   Engine của chúng ta bình tĩnh lôi tấm ảnh caro hồng/đen mờ có sẵn trong VRAM ra vẽ bình thường (vì Handle luôn hợp lệ).
*   Lệnh vẽ tiếp tục, viên đạn biến thành hình vuông caro lơ lửng trên màn hình.

---

## 5. Bước 4: Thực thi Viewport 2

Khung hình của Viewport 1 đã render xong, Engine chuyển sang `RootGraph_Viewport_2`.

*   Mục đích của Viewport này là chiếu cận cảnh nhân vật (để user dễ bề chỉnh sửa).
*   ECS đã tính toán sẵn sự tối ưu này. Lệnh vẽ nó gửi xuống chỉ là 1 `DrawBatch` chứa: `DrawCommand(pipeline: PipelineHandle(2), bind: [TextureHandle(5)], action: Procedural(3))`.
*   Engine thấy lệnh này, nó lôi `TextureHandle(5)` (tấm ảnh Offscreen mà nó vừa cực khổ vẽ ở hành động 3.1) ra, áp Shader phóng to và in thẳng ra Viewport 2.
*   **Kết quả:** Dù nhân vật có 100 cái xương, GPU không tốn thêm 1 giọt mồ hôi nào để tính toán lại cấu trúc nhân vật. Nó chỉ thực hiện 1 lệnh vẽ Procedural Quad xài lại ảnh cũ rẻ bèo.

---

## 6. Bước 5: Chốt Frame (Present)

Sau khi duyệt hết mảng chứa 2 RootGraph:
*   GPU Engine gọi hàm `present()`.
*   Khung hình được đẩy từ VRAM lên màn hình (Monitor) của User.
*   Đồng hồ điểm 16 mili-giây (Mượt mà 60fps).
*   Trở về Bước 1, chờ đợi Render Graph của Frame tiếp theo.
