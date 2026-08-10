# Vòng Đời Tài Nguyên & Tích Hợp UI (Lifecycle & Integration)

Tài liệu này giải đáp các vấn đề cốt lõi về hiệu suất, quản lý tài nguyên (Cache), quá trình giải mã (Decoding), và cách kết nối kết quả render cuối cùng ra giao diện người dùng (Editor UI).

---

## 1. Mối Quan Hệ ECS - GPU Engine (Khởi tạo & Gửi lệnh)

*   **Khởi tạo 1 lần duy nhất (Singleton Lifecycle):** GPU Engine (`ifol-gpu`) là một cỗ máy nặng nề. Khi phần mềm vừa bật lên, Engine được khởi tạo đúng 1 lần duy nhất và nằm chờ sẵn trong RAM. 
*   **Gửi 1 lần duy nhất mỗi khung hình:** Đúng như bạn nhận định, ở mỗi nhịp đồng hồ (ví dụ: mỗi 16ms), ECS sẽ tổng hợp tất cả các Render Graph (cho mọi Viewport) thành một cục dữ liệu duy nhất và **gửi 1 lần duy nhất** xuống GPU. Việc này triệt tiêu hoàn toàn độ trễ giao tiếp (overhead) giữa CPU và GPU.
*   **2 Render Graph trả về cái gì?** Khi gửi 2 Root Render Graph (cho Màn hình chính và Màn hình Preview nhỏ), GPU không trả về "2 Frame thời gian". Nó vẽ ra **2 kết quả ảnh (Texture/Surface) khác nhau** ngay trong cùng 1 mili-giây đó.

---

## 2. Quản Lý File Khác (Video, 3D, v.v.)

**Nguyên tắc:** GPU Engine hoàn toàn **KHÔNG BIẾT** cách đọc file `.mp4`, `.obj` hay `.html`. GPU Engine chỉ biết đọc mảng byte thô (Raw Pixel).

Vậy ai giải mã video?
*   **Hệ thống AssetManager (Nằm cùng tầng với ECS):** Sẽ chịu trách nhiệm chạy FFmpeg, giải mã Video lấy ra khung hình (Seek frame), giải mã file 3D thành lưới Vertex.
*   **Luồng đi:** `AssetManager (Đọc MP4) -> Ép ra mảng byte (Raw RGBA) -> ECS chèn mảng byte đó vào lệnh yêu cầu tải lên VRAM -> GPU Engine nhận mảng byte và biến thành Texture`.
*   **Tính mở rộng:** Vì tách biệt như vậy, tương lai nếu bạn muốn hỗ trợ định dạng Lottie hay SVG, bạn chỉ cần viết thêm hàm giải mã trên CPU (bên AssetManager). GPU Engine không cần sửa đổi bất kỳ dòng code nào.

---

## 3. Chiến Lược Thu Dọn Rác (Cache Eviction & Performance)

Bạn đã chỉ ra một điểm yếu chí mạng rất chính xác: *"Nếu GPU tự động xóa ảnh sau 10 phút không dùng (Thuật toán LRU thuần túy), thì khi User chà xát chuột (Scrub) Timeline qua lại thật nhanh, việc nạp lại ảnh liên tục sẽ gây đứng máy"*.

Vì vậy, chiến lược dọn rác phải kết hợp cả 2:

### 3.1. Chủ Động Từ ECS (Deterministic Eviction) - Ưu tiên 1
ECS là kẻ nắm giữ Timeline, nó biết chính xác "tương lai".
*   Khi User kéo thanh Timeline rời xa một đoạn Video, ECS *biết chắc* là đoạn Video đó không còn xuất hiện nữa.
*   Lúc này, ECS chủ động gửi một chỉ thị (Command) xuống GPU: `"Hãy xóa ngay Texture ID: video_01 khỏi VRAM"`.
*   Đây là cách quản lý hoàn hảo nhất vì nó chính xác tuyệt đối.

### 3.2. Bị Động Bằng LRU (Fallback) - Ưu tiên 2
*   Vẫn phải giữ cơ chế LRU ở GPU Engine như một lớp bảo vệ cuối cùng (Safety Net).
*   Tại sao? Vì giả sử User ném 100 cái Video 4K vào cùng một thời điểm trên Timeline (xếp chồng lên nhau). ECS không thể ra lệnh xóa cái nào vì cái nào cũng đang hiển thị.
*   Lúc này VRAM quá tải, GPU Engine buộc phải dùng quyền sinh sát cuối cùng: Tự động đá các Texture ra khỏi VRAM dựa trên LRU để chống Crash (Thà ứng dụng chạy giật một chút khi nạp lại, còn hơn là văng ứng dụng).

---

## 4. Render Xong Thì Kết Quả Đi Về Đâu? (UI Integration)

Câu hỏi rất hay: *`RenderRequestComponent` render xong thì Editor đọc từ đâu để hiển thị ra cho người dùng?*

Cách kết nối giữa lõi Rust và giao diện (Ví dụ: Tauri/Svelte) thường đi theo 2 hướng:

### Hướng 1: Native Surface (Nhanh Nhất)
*   Editor UI (Svelte) tạo ra một khoảng trống (một cái thẻ `<div>` hoặc `canvas`).
*   Bên Svelte gọi API lấy địa chỉ vùng nhớ phần cứng của khoảng trống đó (OS Window Handle).
*   Truyền Handle đó cho `RenderRequestComponent`.
*   GPU Engine (`ifol-gpu`) nhận Handle này và ép card đồ họa in kết quả (Swap Chain) **thẳng vào vùng trống đó trên màn hình** bỏ qua hoàn toàn trình duyệt (Webview).
*   **Đặc điểm:** Tốc độ bàn thờ, độ trễ bằng 0, vì Rust vẽ trực tiếp lên màn hình hệ điều hành. (Các phần mềm như Figma/Spline dùng WebGL, nhưng Tauri cho phép ta chọc thủng Webview để xài Native wgpu).

### Hướng 2: Offscreen Texture sang Shared Memory (Đa Dụng)
*   Thay vì vẽ ra màn hình, GPU Engine vẽ kết quả ra một tấm ảnh Ẩn trong VRAM.
*   CPU copy tấm ảnh đó xuống Shared Memory (RAM chung).
*   Bên UI (Svelte) đọc mảng RAM đó, nhét vào thẻ `<canvas>` hoặc `<img>` để hiển thị.
*   **Đặc điểm:** Chậm hơn Hướng 1 do mất công copy từ GPU về CPU, nhưng cực kỳ linh hoạt để làm Editor UI phức tạp (hiệu ứng kéo thả layer UI đè lên video). 

👉 **Thiết kế của chúng ta:** GPU Engine sẽ hỗ trợ cả 2 Output Mode (Xuất ra Surface hoặc Xuất ra Memory). Tùy vào việc `RenderRequestComponent` truyền cấu hình nào xuống, GPU Engine sẽ đưa kết quả về đúng nơi đó. Không hề bị trói buộc!
