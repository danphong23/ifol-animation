# Báo cáo: TC14_GRADING - Cinematic Color Grading & ACES Filmic Tone Mapping

Đây là báo cáo tổng hợp chất lượng render của TC14_GRADING trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 2.3838ms
- **Thời gian Render (Warm/Cached - Các lần sau):** 781.2µs
- **Kết quả ảnh (Thực tế):**

![TC14_GRADING Desktop Render](../outputs/desktop/tc14_grading.png)

- **Kỳ vọng:** Khung cảnh hoàng hôn rực rỡ với đường cong ACES Filmic Tone Mapping, hiệu ứng Split-Toning hòa sắc bóng tím chàm và ánh sáng vàng hổ phách, kết hợp Vignette viền mềm tạo cảm giác điện ảnh đỉnh cao.
- **Mô tả (Vision AI / Đánh giá):** Xác thực toàn diện Pipeline Color Grading & Tone Mapping hậu kỳ của ifol-gpu. Hoàn thành kiểm tra độ chuẩn xác xử lý dải màu động và phân loại màu sắc điện ảnh.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
Chưa chạy riêng test case này trên WebGPU. Canonical offscreen probe Desktop/Web đã
pass exact từng byte, nhưng không thay thế cho pixel parity của TC14.

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Desktop: PASS. Pixel parity riêng của TC14 trên Web chưa được kết luận.
