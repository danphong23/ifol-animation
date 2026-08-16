# Kế hoạch tách module từng bước

Đây là execution plan hiện hành. File
`docs/ifol-gpu-upgrade-plan.md` là architecture backlog dài hạn, không phải
danh sách task chạy nguyên khối.

## Gate nền trước khi bắt đầu

```bash
cargo fmt --all -- --check
cargo check -p ifol-gpu
cargo test -p ifol-gpu --lib
```

Baseline phải được ghi trong
`70-status/88-current-handoff-baseline.md`. Nếu baseline thay đổi, cập nhật
status trước khi bắt đầu task mới.

## Phase A — Tách execution God File

File hiện tại lớn nhất là `src/execution/mod.rs`. Chỉ tách một nhóm trách nhiệm
mỗi lần:

| Task | Trách nhiệm | Test chính | Commit mẫu |
|---|---|---|---|
| A1 | validation và typed validation error | invalid graph, missing resource, usage mismatch | `refactor(execution): extract validation module` |
| A2 | render pass encoding | draw, target, depth, MSAA | `refactor(execution): extract render encoding` |
| A3 | compute pass encoding | dispatch, storage buffer/texture | `refactor(execution): extract compute encoding` |
| A4 | copy encoding | buffer copy, texture copy, range/aspect | `refactor(execution): extract copy encoding` |
| A5 | binding/pipeline helpers | layout, dynamic offset, cache key | `refactor(execution): extract binding helpers` |
| A6 | readback và submission boundary | async readback, row alignment, errors | `refactor(execution): isolate readback boundary` |
| A7 | execution facade | toàn bộ execution regression | `refactor(execution): make module facade explicit` |

Mỗi task phải pass unit test liên quan và regression test của crate trước khi
commit. Chưa tạo `CompiledGraph` mới trong phase này.

## Phase B — Tách graph model

| Task | Trách nhiệm | Test chính |
|---|---|---|
| B1 | graph/node/target model | create graph, add node, nested graph |
| B2 | command model | draw, compute, copy command construction |
| B3 | dependency và flatten | DAG, diamond, nested graph, cycle |
| B4 | usage/hazard analysis | read-after-write, subresource, range |
| B5 | graph facade | toàn bộ graph và execution regression |

Graph tiếp tục flatten ra format hiện tại. Việc đổi sang `CompiledGraph` là task
kiến trúc riêng sau khi extraction đã ổn định.

## Phase C — Tách resources

Tiến độ hiện tại: C1 đã hoàn tất; phần version state của C3 đã được tách
thành `src/resources/versions.rs`; nhóm lookup của C2 đã được tách thành
`src/resources/lookup.rs`, `src/resources/mutation.rs` và
`src/resources/ownership.rs` đã được tách, đều giữ nguyên API/behavior. C5 đã
chốt facade root bằng các re-export explicit; các nested module
Các nested implementation paths không còn là public contract.
Sau C5, chuyển sang Phase D; mỗi nhóm vẫn phải đi qua compile và toàn bộ
regression test trước khi commit.

| Task | Trách nhiệm | Test chính |
|---|---|---|
| C1 | descriptor types | invalid extent, usage, mip, sample count |
| C2 | registry core | lookup, insert, remove |
| C3 | handle/version boundary | stale handle, generation, invalid handle |
| C4 | ownership/lifetime helpers | deferred removal, submission completion |
| C5 | resources facade | toàn bộ resource và execution regression |

Không tách texture/buffer/pipeline thành file riêng nếu chúng vẫn chia sẻ logic
registry/lifetime. Tách theo responsibility, không tách theo danh sách loại dữ liệu.

## Phase D — Color/readback boundary

Tiến độ hiện tại: primitive readback đã được tách khỏi `backend/engine.rs`
thành `backend/readback.rs`, giữ nguyên checked API và public backend export.
Bước raw readback kèm `format` đã được khóa bằng `RawTextureReadback`; tuple
API cũ đã được loại bỏ sau khi migrate consumer. Save/encode boundary cũng đã được tách
thành `backend/texture_save.rs`; hardcoded output policy hiện được cô lập ở
module này. Feature `image-encode` giờ bật mặc định để giữ behavior, nhưng
`--no-default-features --lib` đã compile mà không kéo image vào core GPU.
Boundary dependency đã rõ; bước kế tiếp là đánh giá higher-layer encoder mà
không làm thay đổi checked error contract.

Chỉ bắt đầu sau khi A-C đã pass:

1. audit mọi occurrence của `Rgba8UnormSrgb`;
2. xác nhận format thực tế khi readback;
3. trả raw readback kèm format;
4. tách file encoding khỏi `GpuEngine`;
5. chuyển `image` thành dependency của higher layer hoặc feature riêng;
6. thêm test format mismatch và raw output;
7. chạy lại toàn bộ regression ảnh ở tầng test/engine.

Phase D đã hoàn tất các extraction và contract gate nêu trên. Phase E1 đã
tách regression tests inline khỏi execution facade vào `execution/tests.rs`.
Phase E2 tiếp tục tách regression tests inline khỏi graph facade vào
`graph/tests.rs`; production facade hiện chỉ giữ `RenderTarget` và các
re-export public cần thiết. Phase E3 tách regression tests inline khỏi
extensions facade vào `extensions/tests.rs`; production facade giữ
extension contracts, validation và registries. Các task này chỉ di chuyển
test responsibility và giữ nguyên behavior. Phase E4 tách regression tests
inline khỏi `memory/frame.rs` vào `memory/frame_tests.rs` qua test-only path;
memory production logic và public API không đổi. Bước kế tiếp là E5: audit
`memory/lru_cache.rs`. Phase E5 tách regression tests inline khỏi transient
pool implementation vào `memory/lru_tests.rs`; pool contracts và public API
không đổi. Phase E6 tách regression tests inline khỏi
`memory/ring_buffer.rs` vào `memory/ring_tests.rs`; ring buffer production
logic và public API không đổi. Phase E7 tách regression tests inline khỏi
`memory/submission.rs` vào `memory/submission_tests.rs`; submission identity
và tracker contracts không đổi. Phase E8 tách regression tests inline khỏi
`memory/deferred.rs` vào `memory/deferred_tests.rs`; deferred destruction
queue contract không đổi. Phase E9 tách headless initialization regression
test khỏi root `lib.rs` vào `lib_tests.rs`; crate module declarations và
public facade không đổi. Phase E10 tách `RenderGraphValidationError` khỏi
validation algorithms vào `execution/validation_errors.rs`, giữ nguyên
public re-export và error semantics. Phase E11 tách copy, texture-aspect,
buffer-range và indirect-range helpers khỏi `execution/validation.rs` vào
`execution/validation_copy.rs`; validation behavior không đổi. Bước kế tiếp
là E12: tách render-target/depth validation helpers. Phase E12 tách các
helper này vào `execution/validation_target.rs`; graph validation flow chỉ
orchestrate target/depth checks và node traversal, không đổi error semantics.
Bước kế tiếp là E13: audit bind-group/pipeline-layout validation helpers.
Phase E13 tách slot, dynamic-offset và render/compute pipeline-layout
validation vào `execution/validation_layout.rs`; public error paths và
validation semantics không đổi. Bước kế tiếp là E14: audit graph traversal
validation còn lại. Phase E14 tách node/resource/command traversal khỏi
`execution/validation.rs` vào `execution/validation_node.rs`; validation
facade giờ chỉ giữ flattening, target/depth orchestration và re-export
orchestration. Bước kế tiếp là E15: audit resource registry production file.
Phase E15 tách regression suite khỏi `resources/registry.rs` vào
`resources/registry_tests.rs`; registry production file giờ chỉ giữ state
container, version API và version bump primitive. Bước kế tiếp là E16: audit
resource versioning boundary và public resource facade. Phase E16 chuyển
version getters/bumpers khỏi registry facade vào `resources/versions.rs`,
giữ nguyên `ResourceRegistry` public API. Bước kế tiếp là E17: audit public
resource re-exports và compatibility facade. Phase E17 chuyển toàn bộ
internal crate/bench imports sang canonical `resources::*` facade, giữ
đã đóng nested registry path. E18 đã loại bỏ API compatibility modules khỏi
`api/mod.rs` và migrate builder/engine consumers sang `backend::*`. E19 đã
loại bỏ `render` facade; render implementation canonical nằm trong execution
modules.
Bước kế tiếp là E20: audit các public facade còn lại trước file-size audit.
Phase E20 tách extension resource-usage validation khỏi `extensions/mod.rs`
vào `extensions/validation.rs`, giữ nguyên public validation API. Bước kế tiếp
là E21: audit production modules còn lại và bắt đầu file-size audit. Phase E21
tách flatten output/error types khỏi `graph/graph.rs` vào `graph/flatten.rs`,
giữ nguyên graph public exports. Bước kế tiếp là E22: tiếp tục audit thuật toán
flatten và private state của graph trước khi quyết định split lớn hơn. Phase E22
tách dependency/hazard ordering khỏi `graph/graph.rs` vào `graph/ordering.rs`,
giữ nguyên `RenderGraph::ordered_node_ids` và chỉ mở internal usage helper cần
thiết cho module ordering. Bước kế tiếp là E23: audit resource-usage derivation
còn lại trong graph. Phase E23 tách resource declaration/accessor API khỏi
`graph/graph.rs` vào `graph/resource_usage.rs`, giữ nguyên các public
`RenderGraph::declare_*_usage` methods. Bước kế tiếp là E24: tách effective
usage derivation của command, extension và render target. Phase E24 chuyển
`RenderGraph::effective_resource_usages` vào `graph/resource_usage.rs`, sau đó
F4 chuyển tiếp phần derivation vào `graph/effective_usage.rs`, giữ nguyên
internal call sites và hazard semantics. Bước kế tiếp là E25: audit
remaining graph orchestration và execution boundaries. Phase E25 tách public
executor facade khỏi `execution/mod.rs` vào `execution/executor.rs`, giữ
nguyên `RenderGraphExecutor`, report/profiling types và canonical exports.
Bước kế tiếp là E26: audit execution orchestration/segments boundary. Phase E26
tách execution diagnostics counting (`execution_counts_for_graph` và recursive
declared usage count) khỏi `execution/orchestration.rs` vào `execution/counts.rs`,
giữ nguyên report/profiling behavior. Bước kế tiếp là E27: audit target
resolution và nested/flat compilation orchestration. Phase E27 tách target
view resolution (`TargetViews` và `resolve_target_views`) khỏi
`execution/orchestration.rs` vào `execution/targets.rs`, giữ nguyên screen,
offscreen và MSAA resolution behavior. Bước kế tiếp là E28: audit nested/flat
compile orchestration boundary. Phase E28 tách flat graph compilation và
owner-path/error mapping khỏi `execution/orchestration.rs` vào
`execution/flat_compile.rs`, giữ nguyên flat/nested execution behavior. Bước
kế tiếp là E29: tách nested graph compilation orchestration. Phase E29 đổi tên
boundary còn lại từ `orchestration.rs` sang `nested_compile.rs`, giữ nguyên
nested compilation behavior và compiler call sites. Bước kế tiếp là E30: audit
execution segments/render-pass boundaries. Phase E30 tách render-bundle cache/
preparation và render-pass/draw encoding khỏi `execution/render.rs` vào
`render_bundles.rs` và `render_pass.rs`, giữ nguyên compiler/segment call sites
và draw behavior. Bước kế tiếp là E31: audit execution segment phases. Phase E31
tách ba execution phases khỏi `execution/segments.rs` vào
`execution/non_render.rs`, `execution/prepass.rs` và
`execution/target_segments.rs`, giữ nguyên compiler/segment behavior. Bước kế
tiếp là E32: audit remaining execution kernels và final file-size/facade audit.
Phase E32 tách render, compute và copy command validation khỏi
`execution/validation_node.rs` vào `execution/validation_render.rs`,
`execution/validation_compute.rs` và `execution/validation_copy.rs`, giữ nguyên
validation order và error contract. Bước kế tiếp là E33: tiếp tục audit các
boundary còn lại trong execution/backend/graph theo kích thước và cohesion.
Phase E33 tách readback ticket lifecycle khỏi `backend/readback.rs` vào
`backend/readback_ticket.rs`, giữ nguyên `ReadbackTicket` public path, checked
readback API và raw-byte contract. Bước kế tiếp là E34: audit tiếp backend và
graph files còn lớn theo responsibility/cohesion. Phase E34 tách resource
subresource overlap/hazard matching khỏi `graph/usage.rs` vào
`graph/usage_overlap.rs`, giữ nguyên usage types, facade re-exports và hazard
semantics. Bước kế tiếp là E35: audit tiếp graph/resource/backend facades.
Phase E35 tách `RenderNodePool` storage/mutation khỏi `graph/nodes.rs` vào
`graph/node_pool.rs`, giữ nguyên `graph::{RenderNode, RenderNodePool}` facade
và node lifecycle behavior. Bước kế tiếp là E36: audit resource registry và
backend capability/builder boundaries. Phase E36 tách runtime adapter/device
creation khỏi `backend/builder.rs` vào `backend/builder_build.rs`, giữ nguyên
builder policy, public setters, `build` API và capability validation behavior.
E37 resource descriptor audit được supersede bởi Phase F vì ưu tiên hiện tại là
loại bỏ legacy API khỏi dự án development trước khi tiếp tục tách thêm file.

## Phase E — Public API và cleanup

- chốt facade public;
- ẩn implementation module khi downstream không còn phụ thuộc;
- xử lý compatibility path bằng migration task;
- chạy workspace check/test;
- file-size audit cuối cùng.

## Phase F — Remove legacy compatibility

Sau khi các responsibility boundary chính đã ổn định, ưu tiên chuyển toàn bộ
consumer nội bộ sang canonical API rồi xóa compatibility layer lỗi thời. Phase
F1 đã migrate builder/engine consumers trong source, examples, benches, desktop
tests và docs từ `api::*` sang `backend::*`, giữ nguyên behavior. Phase F2 đã
migrate resource/render consumers, xóa `resources::registry`, `render/`, các
`api` compatibility modules và tuple readback API cũ. F3 đã migrate toàn bộ
consumer khỏi alias execution `execute`/`execute_with_surface`, cập nhật docs
theo canonical contract và chạy crate verification. Bước kế tiếp là F4:
structure/file-size audit. F4 đã tách effective usage derivation khỏi
`graph/resource_usage.rs` vào `graph/effective_usage.rs`; declaration/accessor
API và hazard semantics không đổi. F5 đã tách indirect-buffer validation khỏi
`validation_copy.rs` vào `validation_indirect.rs`, giữ nguyên validation
errors và internal re-export. F6 đã tách execution report/profiling result
types khỏi `execution/executor.rs` vào `execution/report.rs`, giữ nguyên public
exports và execution behavior. F7 đã tách texture-copy/mip/aspect/format
validation khỏi `validation_copy.rs` vào `validation_texture.rs`, giữ nguyên
validation order và error contract. F8 đã tách extension/dispatcher registry
state và lookup khỏi `extensions/mod.rs` vào `extensions/registry.rs`, giữ
nguyên public exports và registration behavior. F9 đã tách flatten traversal và
flattened dependency ordering khỏi `graph/graph.rs` vào `graph/flattening.rs`,
giữ nguyên graph model, public construction API và hazard semantics. F10 đã tách
bundle cache identity calculation khỏi `execution/render_bundles.rs` vào
`execution/bundle_key.rs`, giữ nguyên cache key behavior và internal test
surface. F11 đã tách transient texture/buffer pool thành
`memory/texture_pool.rs` và `memory/buffer_pool.rs`; `memory/lru_cache.rs`
giữ facade re-export nhỏ, còn submission-gated reuse semantics và regression
tests không đổi. F12 đã tách resource conflict matching thành
`graph/buffer_overlap.rs` và `graph/texture_overlap.rs`; `graph/usage_overlap.rs`
giữ conflict policy và facade điều phối, không đổi hazard semantics. F13 đã
audit và sửa các tài liệu còn mô tả alias execution, raw insertion,
`src/render` facade và backend compatibility path đã bị loại bỏ. Bước tiếp theo
là F14: loại bỏ `FrameContext::seal` legacy convenience API, migrate regression
tests và TC96 desktop evidence sang `seal_with_deferred_textures`; seal
semantics và deferred lifetime không đổi. Bước tiếp theo là F15: tiếp tục
structure/file-size audit. F15 đã tách draw/bind-group/mesh/indirect command
encoding khỏi `execution/render_pass.rs` vào `execution/draw.rs`; render pass
lifecycle và graph traversal không đổi. Bước tiếp theo là F16: tiếp tục audit
production hotspots còn lại. F16 đã tách readback regression suite khỏi
`backend/readback.rs` vào `backend/readback_tests.rs`; raw readback contract,
format mapping và ticket behavior không đổi. Bước tiếp theo là F17: tiếp tục
audit production hotspots còn lại. F17 đã xử lý hai warning Rust còn lại trong
`benches/compute_benchmarks.rs` (unnecessary `mut` và unhandled `poll` result),
không đổi benchmark behavior. Bước tiếp theo là F18: tiếp tục audit
production hotspots còn lại. F18 đã tách bốn profiling entrypoints khỏi
`execution/executor.rs` vào `execution/executor_profiling.rs`; executor core,
reporting và public method signatures không đổi. Bước tiếp theo là F19: tiếp
tục audit production hotspots còn lại. F19 đã tách timestamp profiling
regression suite khỏi `api/profiling.rs` vào `api/profiling_tests.rs`; profiling
contract và submission-gated pool behavior không đổi. Bước tiếp theo là F20:
tiếp tục audit production hotspots còn lại. F20 đã tách draw/compute/copy
command model khỏi `graph/commands.rs` vào ba module sibling tương ứng; public
graph exports và command builder behavior không đổi. F21 đã tách resource
registry regression suite khỏi `resources/registry_tests.rs` thành
`resources/registry_version_tests.rs`, `resources/registry_descriptor_tests.rs`
và `resources/registry_ownership_tests.rs`; registry behavior và public API
không đổi. Bước tiếp theo là F22: tiếp tục audit production hotspots còn lại.

## Không nằm trong đợt tách file đầu tiên

- shader reflection;
- material system;
- color system;
- video/audio/editor;
- automatic pipeline generation;
- allocator redesign;
- đổi semantics của render graph;
- tối ưu hiệu suất chưa có benchmark chứng minh.
