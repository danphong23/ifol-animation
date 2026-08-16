# IFOL GPU: bind-group limit theo capability

## Thay đổi

Bind-group state cache không còn dùng mảng cố định bốn slot. Khi execute checked,
executor lấy `engine.capabilities().max_bind_groups` để cấp cache và validate
slot theo đúng device.

Điều này giữ được graph model cho device có limit khác nhau và loại bỏ một
hard-code không cần thiết trong core.

`RenderGraphExecutor::validate` không nhận device nên dùng
`wgpu::Limits::default().max_bind_groups` làm policy độc lập. Host cần chẩn đoán
đúng adapter trước submit có thể dùng `validate_with_device`; các API execute
checked cũng dùng capability snapshot thực tế.

## Invariant

- slot hợp lệ là `0 .. max_bind_groups` của device;
- cache render, compute, render bundle và segmented path cùng dùng một limit;
- slot vượt limit trả `InvalidBindGroupSlot { slot, max_slots }` trước submit;
- không thêm giới hạn 4 slot trong command model.

## Test gate

Unit test kiểm tra slot 7 hợp lệ khi limit là 8 và slot 4 không hợp lệ khi limit
là 4. Full execution regression tiếp tục bảo vệ các graph hiện có.
