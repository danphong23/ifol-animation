# Surface initialization contract

Khi builder nhận `with_surface`, adapter phải cung cấp được
`SurfaceConfiguration`. Nếu không, `build()` trả `GpuError::SurfaceUnsupported`
thay vì tạo engine có surface nhưng thiếu format/config.

Headless engine không có surface vẫn hợp lệ và dùng offscreen target. Việc
acquire/present, surface lost/outdated và lifecycle window vẫn thuộc host/platform
integration; core chỉ đảm bảo không tạo trạng thái surface nửa hoàn chỉnh.
