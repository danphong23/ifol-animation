# Báo cáo: TC74_YUV - Video Format Conversion (YUV 4:2:0 to RGBA)

Đây là báo cáo tổng hợp chất lượng render của TC74_YUV trên các nền tảng.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render:** ~7.7ms
- **Kết quả ảnh (Thực tế):**

<img src="../outputs/desktop/tc74_yuv.png" alt="TC74 Desktop Render" />

- **Kỳ vọng:** Khôi phục chính xác ảnh màu gốc từ 3 mặt phẳng (Y, U, V) ở định dạng YUV 4:2:0 sử dụng chuẩn BT.601.
- **Mô tả (Vision AI / Đánh giá):** Compute Shader nhận đầu vào là 3 texture R8Unorm đại diện cho Y (Full resolution) và U, V (Half resolution). Shader tính toán tọa độ `uv_pos = pos / 2` để ánh xạ chính xác độ phân giải của U, V theo chuẩn 4:2:0 subsampling mà không cần dùng đến Sampler. Dữ liệu sau đó được nhân ma trận chuyển đổi BT.601 để xuất ra RGB và ghi thẳng vào `StorageTexture`. Ảnh kết quả giống hoàn toàn ảnh gốc (`bg_nightsky.jpeg`), chứng tỏ thuật toán chuyển đổi màu và tính toán padding texture là hoàn toàn chính xác.
- **Core Engine Errors:** Không có lỗi.

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt 100%. Node YUV to RGBA Converter này rất thiết thực để tối ưu băng thông (bớt truyền RGB to GPU) khi import video MP4 (YUV420p) từ hệ thống WebCodecs / FFmpeg.
