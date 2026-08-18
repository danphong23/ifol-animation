# Hướng dẫn: execute và quản lý lifecycle

## Execute graph

```text
RenderGraph
    → validate_with_device(...)
    → execute_checked(...)
    → SubmissionIndex
    → host poll/completion policy
```

`execute_checked` validate graph, flatten nested graph, kiểm tra resource/
capability rồi mới encode và submit. API trả `wgpu::SubmissionIndex`; host phải
chờ submission hoàn tất trước khi reuse hoặc giải phóng resource. Lỗi trả về
typed; core không bỏ qua node hoặc resource thiếu.

## Surface

- Dùng `try_resize_surface` và xử lý `SurfaceResizeError`.
- Dùng `reconfigure_surface` khi surface bị lost/outdated theo policy của host.
- Present/acquire/retry vẫn do host/event loop điều khiển.

## Readback

Dùng `begin_texture_readback_checked` cho async hoặc
`read_texture_to_raw_with_format_checked` cho synchronous readback. Cả hai
trả `ReadbackError`; format phải được truyền rõ ràng.

Readback là điểm giao giữa core và tầng media. Core trả raw bytes, kích thước và
format thực tế; host quyết định decode input, color/alpha policy, chuyển đổi
định dạng và encode PNG/JPEG/EXR/video. Không dùng surface hoặc canvas preview
để thay thế raw readback khi cần output canonical.

## Resource lifetime

Host phải giữ resource sống trong lúc graph đang dùng. Với owned texture và
transient resource, chỉ reuse hoặc destroy sau submission completion thông qua
`SubmissionTracker`, `FrameContext` hoặc deferred destruction queue.

## Profiling

Timestamp profiling là opt-in. Nếu adapter không hỗ trợ timestamp, dùng
`ExecutionReport` hoặc CPU tracing; thiếu profiling capability không phải lỗi
render.
