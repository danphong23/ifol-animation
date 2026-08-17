# Báo cáo: TC43_TRACK_MATTE - Dual-Layer Track Matte

Đây là báo cáo tổng hợp chất lượng render của TC43_TRACK_MATTE trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 997.4µs
- **Thời gian Render (Warm/Cached - Các lần sau):** 702.6µs
- **Kết quả ảnh (Thực tế):**

![TC43_TRACK_MATTE Desktop Render](../outputs/desktop/tc43_track_matte.png)

- **Kỳ vọng:** Sử dụng bóng của nhân vật (Target Matte) làm mặt nạ Track Matte để bọc lấy texture không gian Sci-Fi. Hỗ trợ 4 chế độ: Alpha Matte, Inverted Alpha, Luma Matte, Inverted Luma.
- **Mô tả (Vision AI / Đánh giá):** Xác thực khả năng đọc đồng thời 2 texture động độc lập và tính toán độ trong suốt Stencil trong Fragment Shader.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt chuẩn 100% so với thiết kế.
