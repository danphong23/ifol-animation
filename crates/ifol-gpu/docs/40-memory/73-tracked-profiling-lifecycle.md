# IFOL GPU: lifecycle profiling có kiểm soát submission

## Vấn đề

`TimestampQueryPool` dùng lại query slot qua nhiều frame. Nếu host submit một
graph rồi reset pool trước khi GPU hoàn tất, query đang được encode có thể bị
ghi đè bởi frame sau. `mark_submitted` và `reset_after` đã có primitive để chặn
lỗi này, nhưng API primitive dễ bị gọi thiếu bước.

## Contract

Executor cung cấp API tracked cho profiling toàn graph:

```text
validate
  → allocate span
  → encode timestamp + graph + resolve
  → tracker.begin()
  → profiler.mark_submitted(id)
  → queue.submit
```

Kết quả chứa `ProfiledExecution` với `report.submission` là submission index
của wgpu, `span` là query range, và `tracking_submission` là `Some(SubmissionId)`.
Host phải báo completion bằng `tracker.mark_completed(id)`, sau đó gọi
`profiler.reset_after(&tracker)`.

Trước completion, pool trả `InFlight` khi cấp span mới và `reset_after` trả
`Ok(false)`. Nếu backend không hỗ trợ timestamp encoder, API trả lỗi typed và
không submit command buffer dở dang.

## Phạm vi cố ý không tự động hóa

Core không tự poll device, map resolve buffer, suy ra thời điểm GPU hoàn tất,
hay quản lý nhiều pool theo frame. Những việc đó phụ thuộc event loop, surface
và chính sách host. API không tracked vẫn tồn tại cho host đã có lifecycle riêng.

## Test gate

- graph profiling thông thường vẫn có fallback typed khi backend thiếu capability;
- tracked profiling đặt pool ở trạng thái in-flight;
- reset bị từ chối trước completion và thành công sau `mark_completed`;
- wasm compile check không phụ thuộc backend runtime cụ thể.
