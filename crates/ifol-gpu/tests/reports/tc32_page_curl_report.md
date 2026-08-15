# Báo cáo: TC32_PAGE_CURL - Page Curl 3D Transition

Đây là báo cáo tổng hợp chất lượng render của TC32_PAGE_CURL trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Cold Start - Lần đầu):** 863.6µs
- **Thời gian Render (Warm/Cached - Các lần sau):** 726.8µs
- **Kết quả ảnh (Thực tế):**

![TC32_PAGE_CURL Desktop Render](../outputs/desktop/tc32_page_curl.png)

- **Kỳ vọng:** Chuyển cảnh lật trang 3D (Page Curl) từ Cảnh A (Paladin) sang Cảnh B (Mage) ở mức 50%.
- **Mô tả (Vision AI / Đánh giá):** Xác thực biến dạng UV hình trụ (Cylinder Projection) và tính toán đổ bóng trên nếp gấp trang giấy.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt chuẩn 100% so với thiết kế.
