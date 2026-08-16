# Baseline và điểm bàn giao hiện tại

Tài liệu này dùng để bắt đầu chat/task mới mà không mất ý định hoặc trạng thái.

## Trạng thái kiểm tra gần nhất

Đã xác nhận trong workspace:

```text
cargo check -p ifol-gpu              PASS
cargo test -p ifol-gpu --lib        114 passed, 0 failed
cargo test -p ifol-gpu --no-default-features --lib 113 passed, 0 failed
cargo check -p ifol-gpu --examples --benches PASS (default features)
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

- `src/execution/tests.rs` chứa regression suite lớn nhưng đã tách khỏi
  execution facade;
- `src/graph/tests.rs` chứa graph regression suite; `src/graph/mod.rs` hiện
  là production facade nhỏ với `RenderTarget` và các re-export public;
- compatibility facade còn public rộng;
- `image` thuộc feature `image-encode` (bật mặc định), không bắt buộc với
  core build `--no-default-features`;
- save/encode đã tách khỏi engine vào `backend/texture_save.rs`;
- readback contract đã trả raw bytes kèm format qua `RawTextureReadback`;
- chưa có runtime matrix đầy đủ cho Metal, Linux, browser, Android và iOS;
- file `docs/ifol-gpu-upgrade-plan.md` chưa phải execution plan đã cập nhật
  trạng thái.

## Task tiếp theo được phép thực hiện

Chỉ bắt đầu từ [kế hoạch tách module từng bước](../00-foundation/17-incremental-module-splitting-plan.md),
Task E3: audit extensions facade và tách test boundary hoặc responsibility
rõ ràng tiếp theo mà không đổi behavior. Giữ facade public và extension
semantics nguyên vẹn.

Không đồng thời sửa extension semantics, graph behavior, resource behavior
hoặc color behavior trong Task E3.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.
