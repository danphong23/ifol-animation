# Capability requirements và platform policy

`GpuCapabilities` lưu snapshot limits/features của adapter và cung cấp
`validate_requirements`. Builder gọi validation này trước `request_device`, nên
adapter thiếu feature hoặc limit được báo bằng `GpuError::InsufficientCapabilities`
thay vì để lỗi xuất hiện mơ hồ ở bước khởi tạo device.

API không hard-code backend: cùng một requirement policy áp dụng cho Vulkan,
Metal, DX12, GLES hoặc WebGPU; host có thể chọn `with_backends` và chọn policy
fallback dựa trên capability snapshot.

Các platform mục tiêu vẫn cần chạy baseline riêng. Capability validation không
được hiểu là bằng chứng mọi platform có feature parity.
