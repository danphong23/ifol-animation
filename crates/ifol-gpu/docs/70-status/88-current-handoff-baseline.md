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
- `src/memory/lru_tests.rs` chứa transient pool regression suite;
  `src/memory/texture_pool.rs` giữ texture descriptor key và
  `TransientTexturePool`, còn `src/memory/buffer_pool.rs` giữ buffer
  descriptor key và `TransientBufferPool`; `lru_cache.rs` chỉ là facade
  re-export nhỏ để giữ canonical public path;
- `src/memory/ring_tests.rs` chứa ring buffer regression suite; ring buffer
  production module chỉ giữ allocation và submission-gated reset logic;
- `src/memory/submission_tests.rs` chứa submission tracker regression suite;
  production module chỉ giữ submission identity và completion tracking;
- `src/memory/deferred_tests.rs` chứa deferred destruction regression suite;
  production module chỉ giữ queue và completion-gated drain contract;
- `src/lib_tests.rs` chứa headless initialization regression test; root
  `lib.rs` chỉ giữ crate module declarations và public facade;
- `src/execution/validation_errors.rs` chứa typed validation error contract;
  `validation.rs` giữ orchestration và các re-export nội bộ cần cho validation;
- `src/execution/validation_copy.rs` chứa copy-command và buffer-range
  validation helpers;
- `src/execution/validation_indirect.rs` chứa indirect-buffer range và usage
  validation; `validation.rs` giữ re-export nội bộ cho command validators;
- `src/execution/validation_texture.rs` chứa texture-copy, mip/aspect và
  format validation helpers;
- `src/execution/validation_target.rs` chứa render-target và depth/stencil
  validation helpers; `validation.rs` giữ graph-validation orchestration;
- `src/execution/validation_layout.rs` chứa bind-group slot, dynamic-offset
  và render/compute pipeline-layout validation helpers;
- `src/execution/validation_node.rs` chứa node/resource/command traversal
  validation; command-specific checks được delegate sang các validation command
  modules; `validation.rs` hiện là facade/orchestrator nhỏ;
- `src/resources/registry_tests.rs` chứa resource registry/descriptor/ownership
  regression suite; `registry.rs` chỉ giữ state container và version API;
- `src/resources/versions.rs` hiện giữ cả version storage và version behavior;
  registry facade chỉ giữ container/constructor;
- Internal crate, examples, tests và benchmark code đã dùng canonical
  `resources::*` exports; registry implementation không còn public qua nested
  module path;
- `src/api/compatibility.rs` đã được loại bỏ; `api/mod.rs` chỉ còn profiling
  functionality và explicit canonical re-exports;
- `src/render/` compatibility facade đã được loại bỏ; render implementation
  canonical nằm trong `execution/render_pass.rs` và `execution/render_bundles.rs`;
- `src/extensions/validation.rs` chứa validation resource-usage của extension;
  `extensions/mod.rs` giữ contract/context/errors; `extensions/registry.rs`
  giữ extension và dispatcher registry state/lookup;
- `src/graph/flatten.rs` chứa `FlatRenderPlan`, `FlatRenderNode`, dependency và
  flatten error types; `graph/mod.rs` giữ public graph exports;
- `src/graph/flattening.rs` chứa `RenderGraph::flatten` traversal, nested graph
  path collection và flattened dependency/hazard ordering; `graph.rs` chỉ giữ
  graph model và construction API;
- `src/graph/ordering.rs` chứa dependency/hazard ordering của graph;
  `RenderGraph::effective_resource_usages` là internal helper dùng chung;
- `src/graph/resource_usage.rs` chứa resource declaration/accessor API của
  `RenderGraph`; storage hiện được mở ở mức `pub(crate)` cho các graph modules;
- `src/graph/effective_usage.rs` chứa effective usage derivation từ copy/draw/
  compute commands và render target; `resource_usage.rs` chỉ giữ declaration
  và accessor API;
- `src/execution/executor.rs` chứa public `RenderGraphExecutor` facade và
  execution orchestration;
- `src/execution/report.rs` chứa `ExecutionReport`, `ProfiledExecution` và
  `RenderGraphProfilingError`; `execution/mod.rs` giữ public re-export và
  module wiring;
- `src/execution/counts.rs` chứa execution diagnostics counting và recursive
  declared-usage counting;
- `src/execution/targets.rs` chứa target view resolution cho screen, offscreen
  và MSAA; compiler/nested compile dùng chung `TargetViews` internal contract;
- `src/execution/flat_compile.rs` chứa flat graph compilation, owner-path
  resolution và flatten-error mapping;
- `src/execution/nested_compile.rs` chứa nested graph compilation bottom-up;
- `src/execution/render_bundles.rs` chứa bundle update và render-node
  preparation;
- `src/execution/bundle_key.rs` chứa pure bundle cache identity calculation;
  `render_bundles.rs` giữ bundle encode/update lifecycle;
- `src/execution/render_pass.rs` chứa render pass lifecycle, graph pass và draw
  command encoding; `render.rs` facade đã được loại bỏ;
- `src/execution/non_render.rs` chứa execution của extension/copy/compute không
  có render target;
- `src/execution/prepass.rs` chứa prepass extension/copy rồi compute;
- `src/execution/target_segments.rs` chứa ordered target segment render passes;
- `src/execution/validation_render.rs` chứa validation của draw commands;
- `src/execution/validation_compute.rs` chứa validation của compute commands;
- `src/execution/validation_copy.rs` chứa validation của copy commands cùng các
  buffer-range helpers; indirect validation nằm trong `validation_indirect.rs`
  và texture validation nằm trong `validation_texture.rs`;
- `src/backend/readback_ticket.rs` chứa mapping, submission wait và row-padding
  resolution của `ReadbackTicket`; `backend/readback.rs` giữ facade/API bắt đầu
  readback;
- `src/graph/usage_overlap.rs` chứa resource access conflict và subresource/aspect
  overlap matching; `graph/usage.rs` giữ usage types, range constructors và
  facade re-export;
- `src/graph/node_pool.rs` chứa `RenderNodePool` storage, allocation, mutation
  và removal; `graph/nodes.rs` giữ `RenderNode` model/behavior;
- `src/backend/builder_build.rs` chứa runtime adapter/device creation và surface
  configuration; `backend/builder.rs` giữ builder policy/configuration facade;
- builder/engine consumers nội bộ trong source, examples, benches, desktop tests
  và integration docs đã migrate từ `api::*` sang `backend::*`;
- compatibility facades cũ đã được loại bỏ khỏi source; các public domain modules
  hiện là canonical API. Alias execution `execute` và `execute_with_surface`
  cũng đã được migrate toàn bộ consumer sang các API `*_checked` và loại bỏ;
- `image` thuộc feature `image-encode` (bật mặc định), không bắt buộc với
  core build `--no-default-features`;
- save/encode đã tách khỏi engine vào `backend/texture_save.rs`;
- readback contract đã trả raw bytes kèm format qua `RawTextureReadback`; tuple
  readback API cũ đã được loại bỏ;
- chưa có runtime matrix đầy đủ cho Metal, Linux, browser, Android và iOS;
- file `docs/ifol-gpu-upgrade-plan.md` chưa phải execution plan đã cập nhật
  trạng thái.

## Task tiếp theo được phép thực hiện

Chỉ bắt đầu từ [kế hoạch tách module từng bước](../00-foundation/17-incremental-module-splitting-plan.md),
Task F12: tiếp tục structure/file-size audit ở các production hotspot còn
lại; giữ semantics nguyên vẹn và chỉ tách thêm khi boundary responsibility
đã rõ.

Không đồng thời sửa memory semantics, extension behavior, graph behavior,
resource behavior hoặc color behavior trong Task F4.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.
