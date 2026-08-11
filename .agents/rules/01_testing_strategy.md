# Chiến Lược Kiểm Thử (Testing Strategy & Guidelines)

Tài liệu này là **Luật Bắt Buộc** đối với mọi đoạn code được viết trong dự án. Bạn tuyệt đối không được gộp code (merge/commit) nếu chưa có hệ thống Test đi kèm bảo vệ.

---

## 1. Triết Lý Kiểm Thử (Test-Driven Mindset)
Dự án này đề cao sự ổn định tuyệt đối (Zero-crash). Vì vậy, tư duy khi viết code là: **Luôn nghĩ đến Edge Cases (Trường hợp dị biệt) trước khi viết Logic chính.**
*   *Ví dụ:* Khi viết hàm tính toán tọa độ Scale, đừng chỉ test `Scale = 1.0` hay `2.0`. Hãy test `Scale = 0.0`, `Scale = -1.0` (âm), hoặc giá trị vô cực (Infinity).

## 2. Các Cấp Độ Kiểm Thử Bắt Buộc

### 2.1. Unit Tests (Kiểm Thử Mức Hàm/Module)
*   **Vị trí:** Đặt ngay bên dưới file mã nguồn (sử dụng `#[cfg(test)] mod tests { ... }`).
*   **Yêu cầu:** Mọi hàm xử lý toán học (trong `ifol-math`), mọi System tính toán tọa độ (trong `ifol-ecs`) đều phải có ít nhất 1 Unit Test cơ bản và 2 Unit Test cho Edge Cases.
*   **Không phụ thuộc:** Unit Test phải chạy siêu tốc. Khởi tạo các Component giả (Mock) trực tiếp trong RAM, không nạp file từ ổ cứng.

### 2.2. Integration Tests (Kiểm Thử Tích Hợp)
*   **Vị trí:** Đặt trong thư mục `tests/` ở thư mục gốc của mỗi Crate.
*   **Mục tiêu:** Đảm bảo các Crate nói chuyện được với nhau.
*   **Kịch bản ví dụ:** Gửi một JSON Command `AddShape` vào `CommandBus` (`ifol-app-core`), sau đó gọi hàm kiểm tra xem trong `ifol-ecs` World có thực sự sinh ra một Entity Shape với đúng tọa độ hay không.

### 2.3. Snapshot Tests (Kiểm Thử Mù cho GPU Engine)
Làm sao để test tự động (Automated Test) xem GPU vẽ có đúng hình không?
*   **Cơ chế:** Vì `ifol-gpu` là Headless (mù quáng), ta có thể ra lệnh cho nó vẽ 1 frame vào một Texture ảo trên RAM thay vì xuất ra màn hình.
*   **So sánh:** Chụp mảng pixel đó lại, băm (Hash) thành 1 chuỗi string, hoặc so sánh từng pixel với một bức ảnh chuẩn (Golden Image). Nếu lệch màu -> Báo lỗi Test thất bại.

### 2.4. Performance Benchmarks (Kiểm Thử Hiệu Năng)
Ở cấp độ đồ họa hoặc logic lõi (như `ifol-gpu`), hiệu năng là yếu tố sống còn. Chúng ta sử dụng thư mục `benches/` (kết hợp thư viện `criterion`) ở thư mục gốc của Crate để đo đạc thời gian chạy chính xác tới từng micro-giây. Các file tài nguyên test (ảnh `.png`, `.wgsl` mẫu) sẽ được đặt biệt lập tại `benches/assets/` để code test tự động nạp. Lõi `ifol-gpu` tuyệt đối không được phép chứa logic đọc file.

**Các kịch bản Benchmark bắt buộc cho GPU Engine:**
1. **Render Siêu Nhẹ**: Chỉ gọi Clear Screen (Đo độ trễ Overhead gốc của API).
2. **Render Opaque (Z-Buffer)**: Vẽ hàng vạn Object đè lên nhau (Đo tốc độ Culling & Z-Test của phần cứng).
3. **Render Transparent (Alpha Blending)**: Vẽ ảnh trong suốt (Đo chi phí xử lý phép toán hòa trộn màu).
4. **Siêu Phức Tạp (Multi-Pass/Deferred)**: Đồ thị 3-4 bước (G-Buffer -> Shadow -> PostProcess) để đo khả năng đồng bộ tài nguyên.
5. **Pipeline Caching**: Gọi đồ thị nặng 2 lần liên tiếp để chứng minh lần thứ 2 có tốc độ vượt trội (nhờ tận dụng Cache từ WGPU).

## 3. Checklist Dành Cho AI Agent 
Mỗi khi bạn (AI Agent) được yêu cầu tạo một Crate mới hoặc viết một Tính năng mới, bạn **PHẢI TỰ ĐỘNG** thực hiện các bước sau mà không cần User nhắc:
1.  Viết bộ khung struct/function (chưa có ruột).
2.  Viết ngay Unit Test ở dưới cùng file để định nghĩa kết quả mong đợi.
3.  Viết code logic để vượt qua (Pass) cái Test đó.
4.  Tự động chạy `cargo test` để chứng minh với User là code của bạn hoạt động đúng.
5.  **Luật Báo Cáo Trực Quan (Visual Report):** Đối với các bài test có tính chất render hình ảnh (GPU), TRƯỚC KHI thực hiện đo lường hiệu năng (`cargo bench`), bạn **BẮT BUỘC** phải tạo ra một bài test trực quan (ví dụ: `visual_tests.rs`) để xuất kết quả ra file ảnh (PNG). Sau đó, bạn phải nạp các file ảnh này vào một Artifact Báo cáo (Markdown) để User tự dùng mắt đánh giá xem ảnh render có đúng (chính xác) không. Nguyên tắc: *Hiệu suất đứng sau độ chính xác.*
