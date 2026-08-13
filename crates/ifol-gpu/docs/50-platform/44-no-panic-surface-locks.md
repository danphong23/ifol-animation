# Surface lock không panic

`GpuEngine::resize_surface` và `surface_format` không dùng `RwLock::unwrap()`.
Nếu lock surface configuration bị poison, resize được bỏ qua an toàn và format
trả `None`; caller có thể xử lý như surface chưa sẵn sàng.

Đây là policy phù hợp với core library: lỗi lifecycle/surface không được làm
panic toàn bộ ứng dụng host. Các lỗi GPU/validation khác tiếp tục dùng
structured `Result` ở API tương ứng.
