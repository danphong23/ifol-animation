# Frame context

`FrameContext` là orchestration boundary cho một frame:

1. host track transient texture/buffer handles;
2. `seal(submission_id, ...)` trả chúng về pool với cùng completion gate;
3. `reset_after` chỉ mở frame lại sau khi submission hoàn tất.

Context không sở hữu backing `wgpu` object và không tự present. Vì vậy host vẫn
quyết định queue submit, surface present và deferred destruction cụ thể; core
chỉ giữ invariant frame không reuse resource in-flight.
