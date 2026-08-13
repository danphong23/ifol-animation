# Checked surface execution

`RenderGraphExecutor::execute_with_surface_checked` validate graph trước khi
encode khi host render vào `SurfaceTexture`. API trả `Result<SubmissionIndex,
RenderGraphValidationError>`, vì vậy graph thiếu resource hoặc có dependency
không hợp lệ không bị bỏ qua âm thầm trên đường render cửa sổ.

`execute_with_surface` và `execute` vẫn được giữ để tương thích; host mới nên
dùng API checked. Đây là bước migration trước khi loại bỏ silent-skip khỏi API
legacy.
