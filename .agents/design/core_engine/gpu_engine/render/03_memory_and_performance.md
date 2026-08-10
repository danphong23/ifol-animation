# 03. Tối Ưu Hiệu Năng & Bộ Nhớ (Memory & Performance)

GPU Engine chỉ mạnh khi nó xử lý được hàng chục nghìn `DrawCommand` mỗi khung hình mà không làm tràn RAM hệ thống. Tài liệu này định nghĩa 3 Kỹ thuật Sống còn.

---

## 1. Zero-Allocation Uniform Ring Buffer
*   **Vấn đề:** Mỗi `DrawCommand` chứa một mảng byte `uniforms` (tọa độ, scale, quay). Nếu mỗi khung hình GPU Engine phải gọi lệnh xin hệ điều hành cấp phát mảng (Malloc/New) rồi lại xóa đi, Game sẽ bị khựng (Stutter) và Garbage Collector sẽ quá tải.
*   **Giải pháp (Ring Buffer):**
    *   Lúc khởi động, Engine xin 1 cục VRAM trống rỗng cực to (Ví dụ 10MB - chứa được hàng triệu ma trận tọa độ).
    *   Mỗi khi đọc 1 lệnh `DrawCommand`, nó sao chép khối byte `uniforms` nhét vào cục 10MB này, và đẩy con trỏ (offset) nhích lên.
    *   Kết thúc 1 Frame, con trỏ tự quay về mức số 0.
    *   👉 **Kết quả:** Không có lệnh cấp phát bộ nhớ động nào xảy ra trong quá trình Loop. Mượt mà 144fps tuyệt đối.

## 2. Thuật Toán Trục Xuất Texture (LRU Cache)
*   **Vấn đề:** User đẩy 100 video 4K vào Timeline. VRAM chỉ có 8GB. Nhồi hết vào VRAM sẽ gây Crash (OOM).
*   **Giải pháp (Least Recently Used):**
    *   Mỗi `TextureHandle` sẽ có một con tem "Khung hình cuối cùng được sử dụng".
    *   Nếu Engine thấy VRAM sắp đầy (Đạt ngưỡng 90%), nó sẽ quét toàn bộ `TextureHandle`. Những ảnh/video nào **chưa từng được gọi lệnh vẽ trong N giây qua** sẽ bị xóa sạch khỏi VRAM.

## 3. Descriptor-based Texture Cache (Bài học từ dự án cũ)
*   **Vấn đề phân mảnh:** Cấp phát và xóa liên tục các Texture ngẫu nhiên sẽ làm phân mảnh bộ nhớ VRAM.
*   **Sai lầm của Game Engine truyền thống:** Trong Game 3D, vật thể to nhỏ liên tục, họ thường dùng kỹ thuật "Làm tròn lên cấp số nhân của 2" (Power of 2 / Slab Allocator). Ví dụ cần 1000px thì cấp luôn 1024px. Tuy nhiên, áp dụng kỹ thuật này cho phần mềm Video Editor là một **sai lầm** vì độ phân giải video (1920x1080) làm tròn lên (2048x2048) sẽ lãng phí lượng VRAM khổng lồ.
*   **Giải pháp (Exact Match Cache):**
    *   Kế thừa kinh nghiệm vô giá từ kiến trúc `ifol-render` cũ, chúng ta sử dụng `Descriptor-based Cache` (Giống cách Bevy Engine làm).
    *   Trong Video Editor, một bức ảnh hoặc một Video thường có **kích thước cố định không đổi** xuyên suốt hàng nghìn khung hình.
    *   Mỗi khi ECS cần 1 ảnh trung gian (Offscreen), Engine tạo ra một Key `(Width, Height, Format)`. Nếu ảnh này vẽ xong và bị vứt đi (Idle), nó không bị xóa khỏi VRAM mà đưa vào Pool.
    *   Frame tiếp theo, nếu ECS lại cần 1 ảnh có ĐÚNG `(Width, Height, Format)` đó, Engine móc ngay từ Pool ra xài lại. Tỉ lệ tái sử dụng (Hit-rate) trong Video Editor là gần 100% mà không bị lãng phí một pixel padding nào!

## 4. Cập Nhật Nóng (Fast Texture Update - Video)
*   Video thực chất chỉ là 1 tấm ảnh (Texture) thay đổi nội dung 60 lần 1 giây.
*   **Không được phép tạo Texture mới:** Nếu mỗi giây tạo 60 cái `TextureHandle` mới và xóa 60 cái cũ, hiệu năng sẽ chạm đáy.
*   **Giải pháp (In-place Write):** Engine cung cấp hàm `write_texture(handle, byte_array)`. Nó cầm khối byte màu sắc mới và **chép đè thẳng** lên vùng nhớ VRAM của cái `TextureHandle` đang có sẵn. Chi phí khởi tạo vật lý bằng 0. Tốc độ bàn thờ.
