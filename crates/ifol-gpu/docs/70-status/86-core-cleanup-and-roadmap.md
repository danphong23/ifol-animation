# IFOL GPU: boundary, cleanup và roadmap hoàn thiện core

Phạm vi authoritative sau audit được ghi tại
`00-foundation/15-core-boundaries-and-task-map.md`: shader reflection, visual
golden harness và engine domain là phần ngoài core, không phải blocker của graph
kernel. Roadmap cũ phải được đọc theo quyết định phạm vi này.

Operation contract đã bổ sung khai báo `ResourceUsage`, validation range và
dispatcher/context để extension encode vào executor.

## Ba tầng kiến trúc

```text
Graph kernel
  Graph / Node / Usage / Dependency / FlatPlan
        ↓
GPU infrastructure
  Executor / Resource store / Memory / Backend / Surface / Profiling
        ↓
External domain
  Shader / Pipeline / Texture data / Video / Timeline / Editor / Game
```

Graph kernel không biết sprite, video, material hay animation. GPU infrastructure
cung cấp khả năng validate và encode plan lên `wgpu`, nhưng không chứa semantic
domain. Host tạo shader, pipeline, texture, buffer, bind group và dữ liệu domain.

## Cấu trúc thư mục mục tiêu

### Trạng thái migration hiện tại

Graph kernel, resource layer và execution layer đã được tách vật lý sang
`src/graph/`, `src/resources/` và `src/execution/`. `src/render` hiện giữ
re-export facade để bảo toàn public path `ifol_gpu::render::*`. Backend và
extension boundary vẫn đang ở phase migration tiếp theo.

```text
src/
  graph/       # logical graph, usage, dependency, flatten, flat plan
  resources/   # handles, descriptors, registry, versions, ownership
  execution/   # validation, compiler, executor, built-in pass encoding
  backend/     # wgpu engine, capabilities, surface boundary
  memory/      # submission, frame, transient and deferred lifetime
  extensions/  # built-in/custom operation boundary
  api/         # public facade and stable re-exports
```

Migration phải giữ re-export compatibility trong một giai đoạn; không đổi public
path chỉ vì đổi thư mục nội bộ.

## Đã đạt

- graph dependency/hazard, flatten và nested execution;
- render/compute/copy/indirect boundary;
- validation typed trước submit;
- resource version/lifetime và submission-safe memory;
- surface/MSAA/resolve/readback/profiling primitives;
- capability policy và baseline compile/test evidence.

## Chưa đạt

- compatibility insert API và migration fixture chưa hoàn tất;
- draw/compute/copy encoder ở flat-plan, segmented, legacy `compile_graph` và
  render-bundle path đã trả typed error;
- docs migration/status còn stale;
- pipeline layout mới là host metadata, chưa có shader reflection;
- capability/format matrix và runtime matrix đa platform chưa đủ evidence;
- extension boundary cho custom graph operation đã có registry, context, validation
  và dispatch; payload semantic/built-in operation cụ thể vẫn là task riêng.

## Roadmap bắt buộc

1. Đóng băng graph kernel public contract.
2. Tách source tree theo boundary ở trên, giữ re-export tạm thời.
3. Migrate examples/tests sang descriptor API và xóa compatibility path.
4. Loại bỏ silent skip trong encoder hoặc biến thành typed internal error.
5. Chuẩn hóa built-in operation và mở rộng test cho custom extension boundary.
6. Viết guides thực hành và API baseline cho host.
7. Sau đó mới làm reflection, capability matrix và runtime portability.

## Phát triển song song

Engine bên ngoài chỉ phụ thuộc public facade và revision/tag ổn định. Breaking
change phải có migration note và test contract.
- Capability snapshot đã được tách vật lý vào `src/backend/capabilities.rs`.
  `src/api/` tiếp tục re-export type này để giữ public path tương thích. Đây là
  bước đầu của backend boundary; builder/engine và surface policy vẫn là task
  migration riêng.
- Builder và engine backend đã được chuyển vào `src/backend/`; `src/api/` giờ
  chỉ còn profiling cùng các re-export public. Các module graph/execution có thể
  dùng backend boundary mà không phụ thuộc đường dẫn facade nội bộ.
Extension registry/identity boundary đã được tạo tại `src/extensions/` và có
test duplicate/empty ID. Custom operation đã được tích hợp vào node, flat plan,
validation và executor dispatcher; built-in operation cụ thể vẫn không thuộc
phạm vi tự động của core.
Import nội bộ của graph, execution, resources và memory đã chuyển sang module
mới; `src/render` không còn là dependency của các layer lõi và chỉ giữ facade
cho consumer cũ.
Examples, benchmark và integration test của crate cũng đã chuyển sang public
module mới; compatibility facade hiện chỉ còn phục vụ consumer bên ngoài chưa
di trú.
`examples/basic_window.rs` đã migrate pipeline và bind group sang descriptor API;
các example/test fixture còn lại vẫn được theo dõi theo nhóm resource.
`examples/visual_tests.rs` đã migrate toàn bộ texture target sang owned descriptor
API và toàn bộ pipeline sang pipeline-layout descriptor API.
`examples/ultimate_test_suite.rs` cũng đã migrate các helper texture, bind group
và pipeline sang descriptor API; raw consumer còn lại tập trung ở
`comprehensive_test.rs` và benchmark.
Ba fixture đầu tiên trong `comprehensive_test.rs` (clear color, depth và alpha)
đã chuyển texture sang owned descriptor API và pipeline sang layout descriptor;
phần fixture còn lại sẽ tiếp tục migrate theo nhóm test.
Fixture interleaved đã chuyển thêm uniform bind group, pipeline layout metadata
và texture target sang descriptor API.
Fixture garbage-collection đã chuyển texture target và pipeline; mesh registry
vẫn dùng API hiện tại vì core chưa định nghĩa descriptor/usage contract riêng
cho mesh.
Fixture complex-frame và multi-graph-cache cũng đã chuyển texture target và
pipeline sang descriptor API.
Fixture nested-graph compositing đã chuyển hai texture target, hai pipeline và
bind group composite sang descriptor API.
Fixture ultimate-master compositing đã chuyển năm texture, bốn pipeline và ba
bind group sang descriptor API; phần fixture 11 còn lại sẽ xử lý riêng.
Các execution fixture cho copy/compute graph cũng đã migrate buffer, pipeline và
bind group sang descriptor API; raw registry consumer còn lại chủ yếu nằm ở
examples lớn và benchmark.
`TextureCache` alias đã bị xóa vì không còn consumer; code dùng
`TransientTexturePool` phải gọi đúng semantics.
`Extension` node đã có representation trong graph và được flatten theo usage;
executor hiện chủ động trả `UnsupportedExtension` cho node chưa có dispatch.
Mọi draw/compute/copy encoder hiện fail-closed khi thiếu pipeline, bind group,
mesh, buffer hoặc owned texture; test missing-resource đã được thêm.
