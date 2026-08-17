# Báo cáo: TC42_HDR_BLOOM - Full-Frame HDR Bloom & Glow

Đây là báo cáo tổng hợp chất lượng render của TC42_HDR_BLOOM trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 1.4015ms
- **Thời gian Render (Warm/Cached - Các lần sau):** 988.8µs
- **Kết quả ảnh (Thực tế):**

![TC42_HDR_BLOOM Desktop Render](../outputs/desktop/tc42_hdr_bloom.png)

- **Kỳ vọng:** Khắc phục triệt để lỗi Glow bị cắt vuông ở viền Sprite: Tách lớp phát sáng ra toàn khung hình (800x600), áp dụng bộ lọc Wide Gaussian Blur và cộng quang học (Additive Blending) lên nền Sci-Fi.
- **Mô tả (Vision AI / Đánh giá):** Xác thực luồng Multi-Pass Composite 3 giai đoạn: Isolate -> Screen Blur -> Additive Blend.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
Chưa chạy TC42 trên Web. Canonical parity probe `Rgba8Unorm` đã pass exact,
nhưng không đại diện cho toàn bộ HDR/multi-pass shader của TC42.

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Desktop: PASS. Cross-platform pixel parity của TC42: CHƯA ĐỦ BẰNG CHỨNG.
