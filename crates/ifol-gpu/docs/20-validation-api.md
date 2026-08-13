# IFOL GPU: API validation hiá»‡n táº¡i

`RenderGraphExecutor::validate` kiá»ƒm tra graph trÆ°á»›c khi táº¡o command buffer vÃ  tráº£
`RenderGraphValidationError` typed. `execute_checked` dÃ¹ng validation nÃ y rá»“i má»›i
submit.

Các lỗi hiện được kiểm tra gồm:

- resource hoặc node không tồn tại;
- target offscreen có kích thước bằng zero;
- bind-group slot ngoài phạm vi hỗ trợ;
- pipeline, mesh hoặc bind group được command tham chiếu nhưng không có trong registry.

`execute` vẫn được giữ cho compatibility prototype. Host production nên chuyển sang
`execute_checked` để không phụ thuộc behavior silent-skip của execution legacy.

Phạm vi còn thiếu gồm pipeline layout compatibility, resource usage, dynamic offset,
attachment sample/format và dependency/cycle diagnostics. Các phần này là task riêng,
không được coi là đã hoàn thiện chỉ vì graph validation cơ bản đã có.
