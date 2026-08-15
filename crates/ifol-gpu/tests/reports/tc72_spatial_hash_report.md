# Báo cáo: TC72_SPATIAL_HASH - GPU Spatial Hashing & Collision

Đây là báo cáo tổng hợp chất lượng render của TC72_SPATIAL_HASH trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render (Compute Hash + Collision + Render):** ~11ms
- **Kết quả ảnh (Thực tế):**

<img src="../outputs/desktop/tc72_spatial_hash.png" alt="TC72 Desktop Render" />

- **Kỳ vọng:** Hàng ngàn hạt (4096 hạt) va chạm với nhau và không bị lọt ra khỏi khung hình, duy trì khoảng cách tối thiểu dựa trên bán kính.
- **Mô tả (Vision AI / Đánh giá):** Compute Shader sử dụng kỹ thuật Spatial Hashing qua 3 Passes: (1) Clear Grid, (2) Hash particles bằng `atomicAdd` để đếm số lượng hạt trong mỗi ô lưới 32x32, (3) Mô phỏng vật lý kiểm tra 9 ô lân cận (3x3 grid cells) và giải quyết va chạm local mà không bị thắt cổ chai $O(N^2)$. Kết quả các hạt lan tỏa và va chạm mượt mà.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt chuẩn 100% so với thiết kế. Kỹ thuật Spatial Hashing này là xương sống cho các hệ thống mô phỏng Boids, Fluid (SPH), và Particle Collider hạng nặng trong tương lai của phần mềm iFol.
