# IFOL GPU: executor timestamp boundary

`RenderGraphExecutor::execute_checked_with_timestamp` và
`execute_with_surface_checked_with_timestamp` là API opt-in cho profiling toàn
graph. Bản thứ hai nhận `surface_view` để profiling không làm mất đường render
trực tiếp ra cửa sổ. Cả hai thực hiện chuỗi:

```text
validate → allocate span → timestamp(begin) → compile flat graph
        → timestamp(end) → resolve query → submit
```

Kết quả là `ProfiledExecution`, gồm `ExecutionReport` và `TimestampSpan`. Buffer
resolve do host sở hữu; `FrameContext` hoặc host quyết định khi nào map/readback
theo submission index.

## Quy tắc

- API execution thường không tạo query và không có profiling overhead.
- Validation xảy ra trước khi cấp span, nên graph lỗi không tiêu thụ query slot.
- Thiếu `TIMESTAMP_QUERY` không được coi là lỗi render; host dùng
  `ExecutionReport`/CPU tracing fallback.
- Có timestamp query nhưng thiếu `TIMESTAMP_QUERY_INSIDE_ENCODERS` trả typed
  profiling error; không panic và không submit command buffer dở dang.
- Boundary hiện là toàn graph, chưa đo từng pass/node. Pass-level profiling cần
  policy query lifetime và integration sâu hơn với FrameContext.
