# IFOL GPU: API validation hiện tại

`RenderGraphExecutor::validate` kiểm tra graph trước khi tạo command buffer và
trả `RenderGraphValidationError` typed. `validate_with_device` và các API
`execute_checked` dùng capability thực của adapter, gồm giới hạn bind-group.

Các lỗi nền tảng được kiểm tra gồm resource/node thiếu, target không hợp lệ,
bind-group slot, pipeline/mesh/bind group thiếu, copy range/format/aspect,
resource usage, dependency và cycle.

`execute` vẫn được giữ cho compatibility prototype; host production nên dùng
`execute_checked` hoặc API surface/profile checked để không phụ thuộc silent-skip.

Phạm vi còn thiếu gồm pipeline layout compatibility đầy đủ, dynamic offset,
attachment sample/format matrix theo backend và diagnostics giàu ngữ cảnh hơn.

