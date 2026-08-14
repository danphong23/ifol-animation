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
capability rồi mới encode và submit. Lỗi trả về typed; core không bỏ qua node
hoặc resource thiếu.

## Surface

- Dùng `try_resize_surface` và xử lý `SurfaceResizeError`.
- Dùng `reconfigure_surface` khi surface bị lost/outdated theo policy của host.
- Present/acquire/retry vẫn do host/event loop điều khiển.

## Readback

Dùng `begin_texture_readback_checked` cho async hoặc
`read_texture_to_bytes_with_format_checked` cho synchronous readback. Cả hai
trả `ReadbackError`; format phải được truyền rõ ràng.

## Resource lifetime

Host phải giữ resource sống trong lúc graph đang dùng. Với owned texture và
transient resource, chỉ reuse hoặc destroy sau submission completion thông qua
`SubmissionTracker`, `FrameContext` hoặc deferred destruction queue.

## Profiling

Timestamp profiling là opt-in. Nếu adapter không hỗ trợ timestamp, dùng
`ExecutionReport` hoặc CPU tracing; thiếu profiling capability không phải lỗi
render.
