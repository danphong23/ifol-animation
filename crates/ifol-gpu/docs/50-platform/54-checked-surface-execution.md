# Checked surface execution

`RenderGraphExecutor::execute_with_surface_checked` validate graph trước khi
encode khi host render vào `SurfaceTexture`. API trả `Result<SubmissionIndex,
RenderGraphValidationError>`, vì vậy graph thiếu resource hoặc có dependency
không hợp lệ không bị bỏ qua âm thầm trên đường render cửa sổ.

Các alias `execute_with_surface` và `execute` đã được loại bỏ khỏi public API.
Host phải dùng `execute_with_surface_checked` hoặc `execute_checked`; mọi graph
đều đi qua validation trước khi encode.
