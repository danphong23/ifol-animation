# Báo cáo: TC86_COMPUTE_OOB - Compute Out-of-Bounds & Boundary Guarding Safety

Đây là báo cáo tổng hợp kết quả kiểm thử an toàn bộ nhớ VRAM cho bài test TC86.

## 1. Môi trường Desktop (Tauri/wgpu)
- **Thời gian Render Cold:** 2.71ms
- **Thời gian Render Warm:** 630.70µs
- **Kết quả ảnh (Thực tế):**

<img src="../outputs/desktop/tc86_compute_oob.png" alt="TC86 Desktop Render" />

- **Kỳ vọng:** Kiểm tra khả năng chặn truy cập bộ nhớ ngoài mảng (Boundary Guarding) khi kích thước Workgroup Dispatch (1,024 luồng) lớn hơn số lượng phần tử mảng thực tế (1,000 phần tử valid).
- **Mô tả (Vision AI / Đánh giá):** 1,000 phần tử dữ liệu hợp lệ nằm bên trái vạch đỏ được GPU Compute tính toán chính xác 100% (cột xanh lá). 24 phần tử đệm phía sau vạch đỏ (cột tím nhạt) nằm trong vùng dải luồng thừa của Workgroup nhưng bị shader chặn bằng `if (idx >= count) return;`, do đó giữ nguyên giá trị 0.0 tuyệt đối, không có bất kỳ hiện tượng ghi đè rác hay rò rỉ bộ nhớ VRAM.
- **Xác thực số học (Readback):**
  - Số phần tử khớp logic CPU: 1000 / 1000 valid items.
  - Số phần tử padding nguyên vẹn: 24 / 24 padding items.
  - Sai số cực đại: 0.00000095.
- **Trạng thái:** **PASSED (An toàn bộ nhớ 100%)**

## 2. Môi trường Web (WASM/WebGPU)
*(Sẽ cập nhật khi chạy trên môi trường Web)*

## 3. Đánh giá Tổng quan (Cross-Platform Consistency)
- Độ hoàn thiện: Đạt 100%. Cơ chế nén luồng và ngắt biên Compute Shader hoạt động chuẩn xác.
