# Báo cáo: TC42_HDR_BLOOM - Full-Frame HDR Bloom & Glow

Đây là báo cáo tổng hợp chất lượng render của TC42_HDR_BLOOM trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 2.0617ms
- **Thời gian Render (Warm/Cached - Các lần sau):** 1.5921ms
- **Kết quả ảnh (Thực tế):**

![TC42_HDR_BLOOM Desktop Render](../outputs/desktop/tc42_hdr_bloom.png)

- **Kỳ vọng:** Khắc phục triệt để lỗi Glow bị cắt vuông ở viền Sprite: Tách lớp phát sáng ra toàn khung hình (800x600), áp dụng bộ lọc Wide Gaussian Blur và cộng quang học (Additive Blending) lên nền Sci-Fi.
- **Mô tả (Vision AI / Đánh giá):** Xác thực luồng Multi-Pass Composite 3 giai đoạn: Isolate -> Screen Blur -> Additive Blend.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
Chưa chạy riêng test case này trên WebGPU. Canonical offscreen probe Desktop/Web đã
pass exact từng byte, nhưng không thay thế cho pixel parity của TC42.

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Desktop: PASS. Pixel parity riêng của TC42 trên Web chưa được kết luận.
