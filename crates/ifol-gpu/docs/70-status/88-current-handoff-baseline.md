# Baseline và điểm bàn giao hiện tại

Tài liệu này dùng để bắt đầu chat/task mới mà không mất ý định hoặc trạng thái.

## Trạng thái kiểm tra gần nhất

Đã xác nhận trong workspace:

```text
cargo check -p ifol-gpu              PASS
cargo test -p ifol-gpu --lib        114 passed, 0 failed
```

Dự án còn có bộ test desktop đến TC105 và các artifact kiểm chứng WebGPU trong
commit gần nhất. Tuy nhiên không được suy ra runtime parity trên mọi platform
chỉ từ compile hoặc test trên Windows.

## Đã có và cần giữ lại

- graph dependency, hazard, cycle và nested flatten;
- render, compute, copy, indirect execution;
- resource descriptor validation;
- generational handle và version invalidation;
- submission-safe lifetime, deferred destruction và ring buffer;
- async readback và typed errors;
- extension registration, validation và fail-closed dispatch;
- desktop/WebGPU test evidence hiện có.

## Chưa sạch hoặc chưa hoàn tất

- `src/execution/mod.rs` còn là God File lớn;
- `src/graph/mod.rs` còn chứa nhiều responsibility;
- compatibility facade còn public rộng;
- `image` vẫn là runtime dependency;
- `GpuEngine::save_texture_to_file_checked` còn gắn file encoding và
  `Rgba8UnormSrgb` vào core;
- readback contract cần khóa rõ format thực tế;
- chưa có runtime matrix đầy đủ cho Metal, Linux, browser, Android và iOS;
- file `docs/ifol-gpu-upgrade-plan.md` chưa phải execution plan đã cập nhật
  trạng thái.

## Task tiếp theo được phép thực hiện

Chỉ bắt đầu từ [kế hoạch tách module từng bước](../00-foundation/17-incremental-module-splitting-plan.md),
Task A1: tách validation khỏi `execution/mod.rs` mà không đổi behavior.

Không đồng thời sửa color, public API hoặc graph semantics trong Task A1.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.

