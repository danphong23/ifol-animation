# Ring buffer và an toàn submission

`UniformRingBuffer` không còn expose reset tùy ý. Allocation chỉ được thu hồi
qua:

```rust
ring.reset_after(&submission_tracker, last_submission)
```

API trả `false` nếu submission vẫn đang in-flight và giữ nguyên offset hiện tại.
Khi tracker đã xác nhận submission hoàn tất, API reset về đầu buffer và trả
`true`.

`SubmissionTracker` là identity/completion primitive ở tầng CPU; host hoặc
`GpuEngine` vẫn chịu trách nhiệm cập nhật completion theo cơ chế poll/callback
của backend. Việc gọi reset trước completion là lỗi correctness vì có thể ghi
đè uniform data mà GPU chưa đọc xong.
