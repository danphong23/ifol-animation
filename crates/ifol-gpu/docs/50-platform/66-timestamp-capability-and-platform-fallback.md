# IFOL GPU: timestamp capability và fallback đa nền tảng

## Hợp đồng

`GpuCapabilities::supports_timestamp_queries` phản ánh trực tiếp việc device
được tạo với `wgpu::Features::TIMESTAMP_QUERY`. Đây chỉ là capability snapshot,
không tự bật feature và không đảm bảo host đã tạo query set.

Host phải dùng capability này trước khi chọn profiler GPU timestamp:

```text
supports_timestamp_queries
        ├── có  → host có thể yêu cầu profiler timestamp
        └── không → dùng ExecutionReport hoặc CPU tracing
```

Điều này đặc biệt quan trọng với Web, mobile và các adapter downlevel, nơi
timestamp query có thể không được expose hoặc không phù hợp với policy runtime.
Core không hard-code một backend, không tự hạ yêu cầu ngầm và không biến thiếu
timestamp thành lỗi render thông thường.

## Phạm vi hiện tại

- Đã snapshot capability và có test cả trạng thái có/không có feature.
- Builder đã có policy `with_required_features`, nên host có thể yêu cầu
  timestamp một cách tường minh nếu ứng dụng bắt buộc phải có.
- Query-set allocation, timestamp insertion, resolve buffer boundary và tracked
  submission lifecycle đã có trong core. Việc map/read kết quả, chọn nhiều pool
  theo frame và trình bày dữ liệu profiling vẫn thuộc host.

## Ma trận kiểm chứng

Mỗi backend/platform phải ghi nhận capability thực tế của adapter, kết quả
builder và fallback được chọn. Không suy luận trạng thái macOS/Web/Android/iOS
từ kết quả Windows hiện tại.
