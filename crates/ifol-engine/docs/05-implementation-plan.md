# Kế hoạch triển khai và trạng thái hoàn tất

Đây không phải MVP/prototype roadmap. Mỗi slice phải hoàn chỉnh contract, test
xanh và không để API tạm. Không tạo abstraction chưa có acceptance case.

Slice 1–8 đã hoàn thành và được chứng minh bởi integration tests. Slice 9 đã
được mở rộng thành hardening cuối cùng ở Slice 13; các mục bên dưới là lịch sử
implementation và boundary đã nghiệm thu, không phải danh sách code chưa có.

## Slice 1 — Runtime lifecycle

Xây `EngineBuilder`, `EngineRuntime`, state machine, empty package set, empty
project và `step()` hữu hạn trên `EcsRuntime`.

Acceptance: build/step/reconfigure/shutdown hợp lệ; misuse trả typed error;
runtime rỗng deterministic.

## Slice 2 — Package identity, resolver và package contract

Xây stable IDs, manifests, semantic constraints, `EnginePackage`, programmatic
package adapter, deterministic dependency resolver và lock result. Builder phải
resolve toàn bộ manifest trước khi gọi bất kỳ package registration nào, sau đó
đăng ký theo topological lock order.

Acceptance: duplicate, missing, incompatible version, cycle, multiple candidates,
input-order permutation và platform capability đều được test.

## Slice 3 — Transactional registration

Xây contribution staging, validation, atomic commit/rollback và controlled
`RegistrationContext` cho component/resource/system/phase/schema/namespace.

Acceptance: lỗi ở mọi stage không publish partial runtime; failed recompile không
giữ schedule stale executable.

Slice này đồng thời xây typed command/query/event registry generic. Concrete
command chỉ tồn tại trong dev-only package; engine core không chứa domain enum.

## Slice 4 — Root resource providers

Xây owned/bound/derived provider, dependency DAG, explicit initialization và
reverse teardown.

Acceptance: exactly-once init/drop, provider cycle, failure giữa chain, duplicate
resource, missing host binding và shutdown idempotence.

## Slice 5 — Project container và package namespace

Xây manifest/lock parsing, generic storage abstraction và namespace claims. Không
thêm asset/render-specific directory.

Acceptance: directory/memory backend cho cùng semantic result; path traversal,
duplicate namespace, invalid ID/version và unknown namespace được xử lý rõ.

## Slice 6 — Scene/schema/migration

Xây generic records, entity remap, codec/migration registry, transactional load,
snapshot/save và opaque preservation.

Acceptance: round-trip, stale schema, migration chain/gap/failure, reference tới
entity thiếu, duplicate entity/component, unknown record và deterministic output.

## Slice 7 — Reconfiguration

Xây staged add/remove/replace package và safe-boundary swap.

Acceptance: world/project preservation theo policy, rollback runtime cũ, resource
teardown order, schema ownership change và step/reconfigure exclusion.

## Slice 8 — Test package chứng nhận extension

Tạo dev-only package đăng ký component, resource, systems, phases, schema,
migration và namespace; không đưa domain production vào engine.

Acceptance: engine source không đổi khi thêm package thứ hai độc lập; cả hai chạy
deterministic và collision bị từ chối.

## Slice 9 — Hardening nền tảng

Audit public API, panic/unsafe, docs, examples, benchmarks có ý nghĩa, feature
matrix native/WASM và dirty-worktree boundary.

Chỉ đóng engine khi toàn bộ Definition of Done và test map xanh. Sau đó mới xây
Name/Hierarchy/Transform/Render/Shape như package production.

## Slice 10–13 — Contract hardening đã hoàn thành

- package/project closure và lock validation trước registration;
- package-owned scene schema/migration/namespace/provider contract;
- scene replacement, `clear_scene`, explicit `SceneId` và opaque preservation;
- project batch-write boundary và escaped manifest round-trip;
- deterministic 256-package resolver chain và regression tests rollback.

Sau Slice 13, không thêm semantic domain vào engine. Các thay đổi tiếp theo chỉ
là bug fix contract, portability evidence hoặc package độc lập bên ngoài crate.
