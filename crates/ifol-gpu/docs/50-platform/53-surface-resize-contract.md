# Hợp đồng resize surface

`GpuEngine::try_resize_surface(width, height)` là API có kết quả rõ ràng:

- kích thước bằng 0 trả `InvalidSize` và không configure surface;
- engine headless hoặc surface chưa có configuration trả `Unavailable`;
- lỗi khóa nội bộ trả `LockPoisoned`.

`resize_surface` vẫn được giữ để tương thích API cũ nhưng chỉ là wrapper bỏ qua
kết quả. Host/platform nên dùng API `try_` để không mất chẩn đoán lifecycle.
