# Capability requirements và platform policy

`GpuCapabilities` lưu snapshot limits/features của adapter và cung cấp
`validate_requirements`. Builder gọi validation này trước `request_device`, nên
adapter thiếu feature hoặc limit được báo bằng `GpuError::InsufficientCapabilities`
thay vì để lỗi xuất hiện mơ hồ ở bước khởi tạo device.

Sau khi tạo engine, `GpuEngine::adapter_info()` trả thông tin adapter đã được
chọn, gồm backend, vendor, device và driver. Host dùng thông tin này để ghi
runtime matrix, diagnostics và quyết định policy bên ngoài; core không tự đổi
shader/material theo vendor.

API không hard-code backend: cùng một requirement policy áp dụng cho Vulkan,
Metal, DX12, GLES hoặc WebGPU; host có thể chọn `with_backends` và chọn policy
fallback dựa trên capability snapshot.

Các platform mục tiêu vẫn cần chạy baseline riêng. Capability validation không
được hiểu là bằng chứng mọi platform có feature parity.
