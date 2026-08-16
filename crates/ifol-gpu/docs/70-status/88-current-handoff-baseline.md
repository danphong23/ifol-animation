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
- `src/extensions/tests.rs` chứa extension regression suite; extension
  facade hiện giữ contracts, validation và registries;
- `src/memory/frame_tests.rs` chứa frame lifecycle regression suite; frame
  production module chỉ giữ lifecycle logic và public API;
- `src/memory/lru_tests.rs` chứa transient pool regression suite; lru cache
  production module chỉ giữ descriptor keys và pool contracts;
- `src/memory/ring_tests.rs` chứa ring buffer regression suite; ring buffer
  production module chỉ giữ allocation và submission-gated reset logic;
- `src/memory/submission_tests.rs` chứa submission tracker regression suite;
  production module chỉ giữ submission identity và completion tracking;
- `src/memory/deferred_tests.rs` chứa deferred destruction regression suite;
  production module chỉ giữ queue và completion-gated drain contract;
- `src/lib_tests.rs` chứa headless initialization regression test; root
  `lib.rs` chỉ giữ crate module declarations và public facade;
- `src/execution/validation_errors.rs` chứa typed validation error contract;
  `validation.rs` giữ algorithms và re-export compatibility;
- `src/execution/validation_copy.rs` chứa copy, texture-aspect, buffer-range
  và indirect-range validation helpers;
- `src/execution/validation_target.rs` chứa render-target và depth/stencil
  validation helpers; `validation.rs` giữ graph-validation orchestration;
- `src/execution/validation_layout.rs` chứa bind-group slot, dynamic-offset
  và render/compute pipeline-layout validation helpers;
- `src/execution/validation_node.rs` chứa node/resource/command traversal
  validation; `validation.rs` hiện là facade/orchestrator nhỏ;
- `src/resources/registry_tests.rs` chứa resource registry/descriptor/ownership
  regression suite; `registry.rs` chỉ giữ state container và version API;
- `src/resources/versions.rs` hiện giữ cả version storage và version behavior;
  registry facade chỉ giữ container/constructor;
- Internal crate và benchmark code dùng canonical `resources::*` re-exports;
  `resources::registry::*` vẫn được giữ nguyên cho downstream compatibility;
- `src/api/compatibility.rs` chứa API builder/engine compatibility modules;
  `api/mod.rs` giữ public exports và re-export các legacy paths;
- `src/render/compatibility.rs` chứa các legacy render handle/registry/compiler/
  graph paths; `render/mod.rs` giữ canonical resource/graph exports;
- `src/extensions/validation.rs` chứa validation resource-usage của extension;
  `extensions/mod.rs` giữ contract, registry và dispatch orchestration;
- `src/graph/flatten.rs` chứa `FlatRenderPlan`, `FlatRenderNode`, dependency và
  flatten error types; `graph/mod.rs` giữ public graph exports;
- `src/graph/ordering.rs` chứa dependency/hazard ordering của graph;
  `RenderGraph::effective_resource_usages` là internal helper dùng chung;
- `src/graph/resource_usage.rs` chứa resource declaration/accessor API của
  `RenderGraph` và effective usage derivation; storage hiện được mở ở mức
  `pub(crate)` cho các graph modules;
- `src/execution/executor.rs` chứa public `RenderGraphExecutor`, execution
  report và profiling result facade; `execution/mod.rs` giữ module wiring và
  compatibility re-exports;
- `src/execution/counts.rs` chứa execution diagnostics counting và recursive
  declared-usage counting; orchestration giữ compile flow;
- `src/execution/targets.rs` chứa target view resolution cho screen, offscreen
  và MSAA; compiler/orchestration dùng chung `TargetViews` internal contract;
- `src/execution/flat_compile.rs` chứa flat graph compilation, owner-path
  resolution và flatten-error mapping; orchestration giữ nested compile flow;
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
Task E29: tách nested graph compilation orchestration, sau đó xử lý từng
boundary một lần mà không đổi behavior. Giữ facade public và render/graph
semantics nguyên vẹn.

Không đồng thời sửa memory semantics, extension behavior, graph behavior,
resource behavior hoặc color behavior trong Task E5.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.
