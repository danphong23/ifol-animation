# Báo cáo: TC91_UNALIGNED_OFFSET - Unaligned Workgroup & Boundary Guarding Safety

Đây là báo cáo tổng hợp chi tiết kết quả kiểm thử an toàn bộ nhớ VRAM khi xử lý mảng phần tử lẻ không chia hết cho kích thước Workgroup (`workgroup_size(64)`) của bài test TC91.

---

## 1. Môi trường & Thông số Thực thi Desktop (Tauri/wgpu)

- **Cấu hình Dispatch:** 5 Workgroups (Tổng 320 luồng GPU đồng thời)
- **Kích thước Mảng Thực tế:** 301 phần tử `f32` (Phần tử lẻ - Unaligned)
- **Kích thước Mảng Bộ Nhớ VRAM Allocated:** 320 phần tử (Bao gồm 19 phần tử Padding)
- **Thời gian Thực thi:** 3.19ms

### Kết quả Ảnh Render (Biểu Đồ Trực Quan):

<img src="../outputs/desktop/tc91_unaligned_offset.png" alt="TC91 Desktop Render" />

- **Giải thích hình ảnh trực quan:**
  - **Dải Cột Xanh Lái/Xanh Lam (301 cột):** Phản ánh 301 phần tử dữ liệu hợp lệ được Compute Shader tính toán chính xác $100\%$ theo công thức $Y = 3.0X + 0.5$.
  - **Vạch Đỏ Ranh Giới (Red Guard Line):** Vạch chặn biên đứng tại vị trí index 301.
  - **Vùng Đáy Sau Vạch Đỏ (19 slots):** 19 luồng GPU dư thừa bị ngắt biên bởi `if (idx >= valid_count) return;`, do đó giữ nguyên mức 0 (không có cột đỏ ghi đè).

---

## 2. Xác Thực Số Học Readback CPU

- **301/301 phần tử valid:** Khớp chính xác $100\%$.
- **19/19 phần tử padding:** Giữ nguyên giá trị `0.0` tuyệt đối.
- **Trạng thái:** **PASSED (An toàn bộ nhớ Boundary Protection 100%)**
