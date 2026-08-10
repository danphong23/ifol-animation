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

## 3. Checklist Dành Cho AI Agent 
Mỗi khi bạn (AI Agent) được yêu cầu tạo một Crate mới hoặc viết một Tính năng mới, bạn **PHẢI TỰ ĐỘNG** thực hiện các bước sau mà không cần User nhắc:
1.  Viết bộ khung struct/function (chưa có ruột).
2.  Viết ngay Unit Test ở dưới cùng file để định nghĩa kết quả mong đợi.
3.  Viết code logic để vượt qua (Pass) cái Test đó.
4.  Tự động chạy `cargo test` để chứng minh với User là code của bạn hoạt động đúng.
