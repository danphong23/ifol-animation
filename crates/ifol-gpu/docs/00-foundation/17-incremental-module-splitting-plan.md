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
`resources::registry::*` và `render::registry::*` vẫn là compatibility paths.
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
thành `backend/readback.rs`, giữ nguyên checked API và compatibility re-export.
Bước raw readback kèm `format` đã được khóa bằng `RawTextureReadback`; tuple
API cũ vẫn có compatibility wrapper. Save/encode boundary cũng đã được tách
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
public facade không đổi. Bước kế tiếp là E10: tách responsibility rõ ràng
trong `execution/validation.rs`.

## Phase E — Public API và cleanup

- chốt facade public;
- ẩn implementation module khi downstream không còn phụ thuộc;
- xử lý compatibility path bằng migration task;
- chạy workspace check/test;
- file-size audit cuối cùng.

## Không nằm trong đợt tách file đầu tiên

- shader reflection;
- material system;
- color system;
- video/audio/editor;
- automatic pipeline generation;
- allocator redesign;
- đổi semantics của render graph;
- tối ưu hiệu suất chưa có benchmark chứng minh.
