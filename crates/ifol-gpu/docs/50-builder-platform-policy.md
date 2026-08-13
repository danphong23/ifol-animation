# Builder platform policy

`GpuEngineBuilder` giữ policy backend/feature/limit một cách explicit:

- `with_backends` chọn backend mask;
- `with_required_features` chọn feature bắt buộc;
- `with_required_limits` chọn limits bắt buộc;
- getter tương ứng cho test harness và diagnostics.

Trước khi tạo device, builder snapshot capability của adapter và validate policy.
Host nhận `GpuError::InsufficientCapabilities` nếu adapter không đáp ứng, nên có
thể fallback policy hoặc chọn backend khác. Đây là contract portable; nó không
thay thế test runtime trên từng Windows/macOS/Linux/Web/Android/iOS target.
