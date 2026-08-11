# Quy Tắc Bắt Buộc: Kiểm Duyệt Trực Quan (Multimodal Visual Testing)

Trong iFol Animation, Đồ họa (GPU) và Giao diện (UI) là cốt lõi. Một hệ thống không thể chỉ chạy nhanh hoặc không báo lỗi (crash) mà quên đi mục tiêu cuối cùng là TÍNH CHÍNH XÁC CỦA ĐIỂM ẢNH (Pixel-perfect).

Do đó, bắt đầu từ Phase 4.4, quy tắc sau đây là **BẮT BUỘC** đối với AI Agent (Antigravity/Cursor/Claude):

1. **Mọi Thuật Toán Đồ Họa Phải Được Render Ra Ảnh:** Bất kể bạn tối ưu hóa `RenderGraphExecutor`, thêm Shader, hay sửa State Caching, bạn BẮT BUỘC phải tạo/cập nhật Test Case để xuất ra file `.png`.
2. **Self-Verification (Tự Kiểm Duyệt Bằng Nhãn Quan AI):**
   - Sau khi chạy Test và sinh ra ảnh, Agent KHÔNG ĐƯỢC báo cáo thành công ngay.
   - Agent **PHẢI SỬ DỤNG CÔNG CỤ ĐỌC ẢNH (`view_file` với chức năng Multimodal)** để trực tiếp "nhìn" vào bức ảnh vừa sinh ra.
   - Agent phải đối chiếu hình ảnh thực tế với kỳ vọng (VD: Tứ giác có ra đúng tứ giác không? Alpha blending có đúng màu không?).
3. **Phân Tích & Báo Cáo:** Nếu ảnh sai, tự động sửa code và lặp lại. Nếu ảnh đúng, trong báo cáo phải chèn bức ảnh đó vào và giải thích chi tiết tại sao nó đúng dựa trên cơ chế render.
