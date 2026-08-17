# Baseline và điểm bàn giao hiện tại

Tài liệu này dùng để bắt đầu chat/task mới mà không mất ý định hoặc trạng thái.

## Trạng thái kiểm tra gần nhất

Đã xác nhận trong workspace:

```text
cargo check -p ifol-gpu              PASS
cargo test -p ifol-gpu --lib        114 passed, 0 failed
cargo test -p ifol-gpu --no-default-features --lib 114 passed, 0 failed
cargo check -p ifol-gpu --examples --benches PASS (default features)
cargo test -p ifol-gpu --test tc_parity_canonical PASS
cargo test -p ifol-gpu --tests -- --test-threads=1 PASS (full desktop suite)
cargo check -p ifol-gpu --no-default-features --tests --examples --benches PASS
```

Full desktop regression suite đã chạy pass với 0 failed; TC95 hiện không có
target trong repository. Dự án còn có bộ test desktop đến TC105 và WebGPU runner.
Canonical parity probe
Desktop/Web đã tạo raw `Rgba8Unorm` giống nhau từng byte; xem
`tests/reports/webgpu_verification_report.md` và
`docs/70-status/89-upgrade-regression-and-parity.md`. Kết quả này chưa chứng
minh pixel parity cho toàn bộ TC98–TC105 hoặc mọi platform.

## Đã có và cần giữ lại

- graph dependency, hazard, cycle và nested flatten;
- render, compute, copy, indirect execution;
- resource descriptor validation;
- generational handle và version invalidation;
- submission-safe lifetime, deferred destruction và ring buffer;
- async readback và typed errors;
- extension registration, validation và fail-closed dispatch;
- desktop/WebGPU test evidence hiện có.

## Ghi chú cấu trúc hiện tại

- execution regression suite đã được tách khỏi execution facade theo các
  boundary validation, encoder, target, copy, compute và execution order;
- `src/graph/tests.rs` chứa graph regression suite; `src/graph/mod.rs` hiện
  là production facade nhỏ với `RenderTarget` và các re-export public;
- `src/extensions/tests.rs` chứa extension regression suite; extension
  facade hiện giữ contracts, validation và registries;
- `src/memory/frame_tests.rs` chứa frame lifecycle regression suite; frame
  production module chỉ giữ lifecycle logic và public API; legacy
  `FrameContext::seal` đã được loại bỏ, mọi consumer dùng
  `seal_with_deferred_textures`;
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
- `src/api/profiling_tests.rs` chứa timestamp profiling regression suite;
  `api/profiling.rs` chỉ giữ profiling contract và pool behavior;
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
- `src/execution/encoder_tests.rs` chứa regression tests cho bind-group slot và
  compute/draw/copy encoder;
- `src/execution/executor_contract_tests.rs` chứa extension-dispatch,
  profiling-entrypoint và execution-report regression tests;
- `src/execution/validation_contract_tests.rs` chứa preflight/validation
  contract tests cho target lookup, indirect range, texture aspect và bundle
  key;
- `src/execution/target_tests.rs` chứa target-size, MSAA resolve/depth
  validation và target execution regression tests;
- `src/execution/command_validation_tests.rs` chứa command/resource usage và
  buffer-range validation regression tests;
- `src/execution/copy_execution_tests.rs` chứa buffer/texture copy execution,
  copy validation và interleaved copy/draw segment regression tests;
- `src/execution/compute_execution_tests.rs` chứa compute-only execution và
  nested-compute ordering regression tests;
- `src/execution/dynamic_offset_tests.rs` chứa descriptor-aware dynamic-offset
  validation regression test;
- `src/execution/attachment_validation_tests.rs` chứa depth-attachment
  validation regression tests;
- `src/execution/pipeline_layout_validation_tests.rs` chứa render/compute
  pipeline-layout validation regression tests;
- `src/execution/indexed_resource_validation_tests.rs` chứa indexed/indirect
  mesh và buffer lookup validation regression tests;
- `src/execution/execution_order_tests.rs` chứa empty-graph, nested-subgraph và
  interleaved draw/copy/compute execution regression tests;
- `src/resources/registry_version_tests.rs` chứa version regression suite;
  `registry_descriptor_tests.rs` chứa descriptor validation suite;
  `registry_ownership_tests.rs` chứa owned-texture/deferred-destruction suite;
  `registry.rs` chỉ giữ state container và version API;
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
- `src/graph/commands.rs` chỉ là facade re-export; `draw_command.rs`,
  `compute_command.rs` và `copy_command.rs` giữ riêng từng command model và
  builder API;
- `src/graph/effective_usage.rs` chứa effective usage derivation từ copy/draw/
  compute commands và render target; `resource_usage.rs` chỉ giữ declaration
  và accessor API;
- `src/execution/executor.rs` chứa public `RenderGraphExecutor` facade và
  validation/execution-report orchestration; `src/execution/executor_profiling.rs`
  chứa các profiling entrypoints của executor;
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
- `src/execution/render_pass.rs` chứa render pass lifecycle và graph pass
  traversal; `src/execution/draw.rs` chứa draw/bind-group/mesh/indirect
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
  readback; `src/backend/readback_tests.rs` giữ readback regression suite;
- `src/graph/usage_overlap.rs` chứa resource access conflict và subresource/aspect
-  overlap facade; `src/graph/buffer_overlap.rs` giữ buffer-range matching và
  `src/graph/texture_overlap.rs` giữ texture/aspect matching; `graph/usage.rs`
  giữ usage types, range constructors và facade re-export;
- `src/graph/node_pool.rs` chứa `RenderNodePool` storage, allocation, mutation
  và removal; `graph/nodes.rs` giữ `RenderNode` model/behavior;
- `src/backend/builder_build.rs` chứa runtime adapter/device creation và surface
  configuration; `backend/builder.rs` giữ builder policy/configuration facade;
- builder/engine consumers nội bộ trong source, examples, benches, desktop tests
  và integration docs đã migrate từ `api::*` sang `backend::*`;
- compatibility facades cũ đã được loại bỏ khỏi source; các public domain modules
  hiện là canonical API. Alias execution `execute` và `execute_with_surface`
  cũng đã được migrate toàn bộ consumer sang các API `*_checked` và loại bỏ;
- `image` chỉ còn là dev-dependency phục vụ test/example support;
- production core không còn save/encode API hoặc `backend/texture_save.rs`;
- readback contract đã trả raw bytes kèm format qua `RawTextureReadback`; tuple
  readback API cũ đã được loại bỏ;
- canonical readback từ owned registry texture tự lấy format từ descriptor;
- Web runner có canonical offscreen parity probe và đo timing riêng;
- chưa có runtime matrix đầy đủ cho Metal, Linux, Android và iOS; browser mới
  có evidence cho WebGPU runner và canonical probe;
- canonical render/export path chưa thuộc `ifol-gpu`; asset decode, renderer
  deterministic và media encoder vẫn là responsibility của higher layer;
- PNG canonical trong `tests/shared_assets/textures/` chỉ là test fixture để
  giảm khác biệt decoder, không phải giới hạn format của core;
- canonical path đã được mô tả rõ trong
  [`18-canonical-render-and-media-output-contract.md`](../00-foundation/18-canonical-render-and-media-output-contract.md):
  tầng ngoài chuẩn hóa input và encode output; core chỉ execute và raw
  readback;
- quyết định boundary đã chốt: decoder, canonical asset bytes, color/alpha
  policy, media encoder và file output đều do tầng ngoài quản lý; không thêm
  API decode/encode hoặc nhánh PNG/JPEG/WebP/video vào `ifol-gpu`;
- canonical render/export là workflow do tầng ngoài sở hữu và điều phối. Tầng
  ngoài có thể dùng `ifol-gpu` làm execution backend, nhưng phải giữ quyền chọn
  canonical input, deterministic policy, readback verification và encoder;
- TC01, TC04, TC05, TC06, TC07 và TC08 đã đạt raw parity tuyệt đối; TC02 và
  TC03 đạt vision/structural/depth parity, còn TC08.5 đạt vision/structural
  parity. TC09 đạt raw parity tuyệt đối và xác nhận cold/warm output không đổi.
  TC10 đạt typed-error/no-panic contract và raw fallback parity tuyệt đối. TC08.5
  là case có điều kiện vì raw pixel parity còn sai khác màu/alpha. TC11 đã đạt
  raw parity tuyệt đối cho ba pass multi-viewport/compositor. TC12 đạt
  vision/structural parity nhưng còn 4 byte sai khác ở 4 pixel. Xem
  [chính sách parity](../00-foundation/19-cross-platform-parity-testing-policy.md)
  và các report TC tương ứng trong `tests/reports/`;
- TC13 đã đạt raw parity tuyệt đối cho graph 4 pass ping-pong Gaussian blur;
  Desktop/Web dùng cùng manifest fingerprint `4f37a8fd4102496e`, cùng 11 draw
  command và output cold/warm không đổi. Xem report TC13 trong
  `tests/reports/tc13_blur_report.md`;
- file `docs/ifol-gpu-upgrade-plan.md` chưa phải execution plan đã cập nhật
  trạng thái.

## Trạng thái handoff

Đợt refactor incremental F21–F31 đã hoàn tất bounded audit. Các production
file còn dài hơn khoảng 180 dòng đều giữ một responsibility rõ ràng; riêng
`graph/tests.rs` là suite nhất quán cho graph scheduling/hazard và không có
boundary đủ độc lập để tách thêm. Không còn task tách file bắt buộc trong
baseline này; thay đổi tiếp theo chỉ nên mở task mới khi có requirement hoặc
behavior mới.

Benchmark target hiện compile sạch, không còn warning Rust trong crate; working
tree phải được kiểm tra lại trước mỗi task mới.

## Hợp đồng với chat/task mới

Chat/task mới phải:

1. đọc file này;
2. đọc `00-foundation/16-current-intent-and-refactor-workflow.md`;
3. đọc task đang chạy trong `17-incremental-module-splitting-plan.md`;
4. kiểm tra Git status để không đụng vào thay đổi ngoài scope;
5. làm đúng một task, test pass rồi commit;
6. cập nhật status/docs nếu contract hoặc baseline thay đổi.
