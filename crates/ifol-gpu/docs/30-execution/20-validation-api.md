# IFOL GPU: API validation hiện tại

`RenderGraphExecutor::validate` kiểm tra graph trước khi tạo command buffer và
trả `RenderGraphValidationError` typed. `validate_with_device` và các API
`execute_checked` dùng capability thực của adapter, gồm giới hạn bind-group.

Các lỗi nền tảng được kiểm tra gồm resource/node thiếu, target không hợp lệ,
bind-group slot, pipeline/mesh/bind group thiếu, copy range/format/aspect,
resource usage, dependency và cycle.

Host production dùng `execute_checked`, `execute_with_surface_checked` hoặc các
biến thể report/profile checked. Core không còn giữ alias `execute` cũ.

Phạm vi còn thiếu gồm pipeline layout compatibility đầy đủ, dynamic offset,
attachment sample/format matrix theo backend và diagnostics giàu ngữ cảnh hơn.
