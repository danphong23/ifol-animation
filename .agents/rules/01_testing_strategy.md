# Chiến Lược Kiểm Thử (Testing Strategy & Guidelines)

Tài liệu này là **Luật Bắt Buộc** đối với mọi đoạn code được viết trong dự án. Bạn tuyệt đối không được gộp code (merge/commit) nếu chưa có hệ thống Test đi kèm bảo vệ.

---

## 1. Triết Lý Kiểm Thử (Test-Driven Mindset)
Dự án này đề cao sự ổn định tuyệt đối (Zero-crash). Vì vậy, tư duy khi viết code là: **Luôn nghĩ đến Edge Cases (Trường hợp dị biệt) trước khi viết Logic chính.**
*   *Ví dụ:* Khi viết hàm tính toán tọa độ Scale, đừng chỉ test `Scale = 1.0` hay `2.0`. Hãy test `Scale = 0.0`, `Scale = -1.0` (âm), hoặc giá trị vô cực (Infinity).

## 2. Các Cấp Độ Kiểm Thử Bắt Buộc

### 2.1. Unit Tests (Kiểm Thử Mức Hàm/Module)
*   **Vị trí:** Đặt ngay bên dưới file mã nguồn (sử dụng `#[cfg(test)] mod tests { ... }`).
*   **Yêu cầu:** Hàm lõi phải có test cho contract và edge case tương ứng. `ifol-ecs` kiểm thử identity/storage/query/scheduler bằng test feature dev-only; system nghiệp vụ được kiểm thử trong feature sở hữu nó, không đặt trong ECS core.
*   **Không phụ thuộc:** Unit test phải chạy hoàn toàn trong RAM khi I/O không phải đối tượng kiểm thử; dùng component/system giả thay vì kéo GPU, project hoặc UI vào test ECS.

### 2.2. Integration Tests (Kiểm Thử Tích Hợp)
*   **Vị trí:** Đặt trong thư mục `tests/` ở thư mục gốc của mỗi Crate.
*   **Mục tiêu:** Đảm bảo các Crate nói chuyện được với nhau.
*   **Kịch bản ví dụ:** Dev-only package đăng ký typed command, component,
    resource, system và phase vào `ifol-engine`; gửi command rồi kiểm tra
    `ifol-ecs` World và commit event đúng contract. Engine test không dùng
    `ShapeComponent` production. JSON chỉ được kiểm tra ở transport boundary.

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
1.  Chốt public contract và acceptance slice trước khi viết production API.
2.  Viết unit/integration/adversarial test cho behavior và rollback của slice.
3.  Viết production implementation hoàn chỉnh để vượt test; không merge stub,
    placeholder, `todo!()` hoặc API tạm được gọi là MVP/prototype.
4.  Chạy fmt, check, clippy `-D warnings`, unit/integration/doc tests và target
    matrix áp dụng cho crate.
5.  **Luật Báo Cáo Test Bắt Buộc:** Đối với các bài test render hình ảnh (GPU), test xong phải lấy ảnh render phân tích lại xem render đúng không đã trước khi nói đến hiệu suất. Sau đó, **BẮT BUỘC** phải tạo 1 báo cáo hoàn chỉnh.

### 5.2. Mẫu Báo Cáo Chuẩn Bắt Buộc
Mọi Test Case khi báo cáo **BẮT BUỘC** phải có cấu trúc:
1.  **Ảnh Render**: Link trực tiếp tới ảnh PNG sinh ra.
2.  **Render Graph**: Phân tích logic đồ thị gốc.
3.  **Fat Graph (Compiled)**: Lệnh thực tế được GPU hiểu.
4.  **Thời Gian Biên Dịch**: Hiệu năng đo được.
5.  **Mô Tả & Check**: Chấm điểm logic Graph và Compiled.
6.  **⚠️ ĐÁNH GIÁ ẢNH RENDER (AI's Self-Analysis):** Agent tuyệt đối không được báo cáo mù quáng kiểu "Đã lưu file thành công". Bạn phải quan sát/tưởng tượng xem với toạ độ shader đó, mesh đó, ảnh sinh ra có đúng là Hình Chữ Nhật / Hình Đa Giác không? Có bị cắt xén không? Có đúng màu và blend không? Nếu shader cấu hình vẽ Quad mà kết quả chỉ ra Tam giác (do sai Vertex Count) thì phải tự phát hiện và sửa code ngay lập tức.

## 4. Acceptance Riêng Cho ifol-engine

Khi triển khai `ifol-engine`, bắt buộc dùng test map tại
`crates/ifol-engine/docs/06-test-and-acceptance-plan.md`. Package resolver, registration,
resource provider, scene load/save và reconfigure đều phải có failure-injection
test chứng minh không partial commit. Report Markdown không thay thế executable
test.
