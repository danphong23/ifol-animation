# Hợp đồng resize surface

`GpuEngine::try_resize_surface(width, height)` là API duy nhất để resize surface
và có kết quả rõ ràng:

- kích thước bằng 0 trả `InvalidSize` và không configure surface;
- engine headless hoặc surface chưa có configuration trả `Unavailable`;
- lỗi khóa nội bộ trả `LockPoisoned`.

Không còn wrapper `resize_surface` kiểu bỏ qua kết quả. Host/platform phải dùng
`try_resize_surface` để giữ chẩn đoán lifecycle typed.
