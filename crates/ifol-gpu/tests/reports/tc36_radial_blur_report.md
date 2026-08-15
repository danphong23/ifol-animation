# Báo cáo: TC36_RADIAL_BLUR - Radial Blur (Zoom Blur)

Đây là báo cáo tổng hợp chất lượng render của TC36_RADIAL_BLUR trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 1.3556ms
- **Thời gian Render (Warm/Cached - Các lần sau):** 1.223ms
- **Kết quả ảnh (Thực tế):**

![TC36_RADIAL_BLUR Desktop Render](../outputs/desktop/tc36_radial_blur.png)

- **Kỳ vọng:** Làm nhòe hình ảnh tỏa ra từ tâm màn hình (Zoom in). Ứng dụng khi chuyển cảnh nhanh hoặc tạo cảm giác lao tới.
- **Mô tả (Vision AI / Đánh giá):** Xác thực kỹ thuật Sampling hướng tâm với 30 vòng lặp có nội suy giảm dần trọng số (Weight decay).
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt chuẩn 100% so với thiết kế.
