# Báo cáo: TC12_CHROMA - Fine Chroma Key Edge Despill & Smooth Alpha Feathering

Đây là báo cáo tổng hợp chất lượng render của TC12_CHROMA trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 22.5482ms
- **Thời gian Render (Warm/Cached - Các lần sau):** 1.3637ms
- **Kết quả ảnh (Thực tế):**

![TC12_CHROMA Desktop Render](../outputs/desktop/tc12_chroma.png)

- **Kỳ vọng:** 5 đối tượng phức tạp (Paladin, Pháp sư, Cuộn giấy ma thuật, Bình thuốc, Túi vàng) được bóc tách từ phông xanh lá với độ tinh xảo cao, viền xanh được lọc sạch 100%, không bị biến dạng và hòa trộn mềm mại trên nền hoàng hôn.
- **Mô tả (Vision AI / Đánh giá):** Xác thực thuật toán Green Despill Filter và Sub-pixel Alpha Edge Feathering của ifol-gpu. Hoàn thành kiểm tra độ chính xác màu sắc và bóc tách phông xanh.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
Chưa chạy riêng test case này trên WebGPU. Canonical offscreen probe Desktop/Web đã
pass exact từng byte, nhưng không thay thế cho pixel parity của TC12.

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Desktop: PASS. Pixel parity riêng của TC12 trên Web chưa được kết luận.
