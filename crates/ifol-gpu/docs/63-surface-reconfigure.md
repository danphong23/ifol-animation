# Surface reconfigure lifecycle

`GpuEngine::reconfigure_surface()` áp dụng lại `SurfaceConfiguration` hiện tại
và trả `SurfaceResizeError` nếu engine headless, chưa có config hoặc lock bị
poison. Host nên gọi API này sau `CurrentSurfaceTexture::Lost` hoặc
`Outdated`, sau khi đã xử lý kích thước cửa sổ nếu cần bằng
`try_resize_surface`.

Core không nuốt lỗi present/acquire và không tự quyết định policy retry; host
window loop vẫn sở hữu quyết định bỏ frame, resize hay kết thúc ứng dụng.
