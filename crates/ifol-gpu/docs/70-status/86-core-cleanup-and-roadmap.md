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

Graph kernel, resource layer, execution layer, backend và extension boundary đã
được tách vật lý sang các module riêng. `src/render` đã bị loại bỏ; `src/api`
chỉ giữ profiling primitives và một số public re-export canonical.

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

Các public path đã được migrate về domain module canonical. Re-export chỉ còn
được giữ khi nó là một phần của public facade hiện hành, không phải để duy trì
API legacy đã bị loại bỏ.

## Đã đạt

- graph dependency/hazard, flatten và nested execution;
- render/compute/copy/indirect boundary;
- validation typed trước submit;
- resource version/lifetime và submission-safe memory;
- surface/MSAA/resolve/readback/profiling primitives;
- capability policy và baseline compile/test evidence.
- mọi resource registration nội bộ đi qua descriptor contract; raw insertion API
  đã bị xóa khỏi `ResourceRegistry`.
- draw/compute/copy encoder ở flat-plan, segmented, `compile_graph` và
  render-bundle path đều fail-closed bằng typed error.

## Chưa đạt hoặc thuộc phạm vi ngoài core

- pipeline layout mới là host metadata, chưa có shader reflection; reflection
  thuộc shader/tool layer bên ngoài;
- capability/format matrix và runtime matrix đa platform chưa đủ evidence;
- pass-level profiling, worker scheduling và policy nhiều frame thuộc host;
- semantic built-in operation cụ thể thuộc engine/domain layer.

## Roadmap bắt buộc

1. Đóng băng graph kernel public contract — đã hoàn tất.
2. Tách source tree theo boundary ở trên, giữ lại chỉ các re-export thuộc
   canonical facade — đã hoàn tất.
3. Migrate examples/tests sang descriptor API và xóa compatibility path — đã hoàn tất
   cho toàn bộ consumer nội bộ, gồm cả benchmark.
4. Loại bỏ silent skip trong encoder hoặc biến thành typed internal error — đã hoàn tất.
5. Chuẩn hóa extension boundary và test custom operation — đã hoàn tất ở mức core.
6. Viết guides thực hành và API baseline cho host — đã hoàn tất.
7. Reflection, capability matrix chi tiết và runtime portability tiếp tục ở
   tool/engine/platform layer; core đã cung cấp contract và evidence host hiện có.

## Phát triển song song

Engine bên ngoài chỉ phụ thuộc public facade và revision/tag ổn định. Breaking
change phải có migration note và test contract.
Capability snapshot, builder/engine backend, extension registry/dispatcher,
graph/execution/resources/memory đều đã nằm đúng boundary; `src/api` chỉ còn
profiling facade và re-export cần thiết. Examples, integration tests và benchmarks đều dùng
public descriptor API, không còn raw resource consumer.
`TextureCache` alias đã bị xóa vì không còn consumer; code dùng
`TransientTexturePool` phải gọi đúng semantics.
`Extension` node đã có representation trong graph và được flatten theo usage;
executor hiện chủ động trả `UnsupportedExtension` cho node chưa có dispatch.
Mọi draw/compute/copy encoder hiện fail-closed khi thiếu pipeline, bind group,
mesh, buffer hoặc owned texture; test missing-resource đã được thêm.

Mesh registry cleanup: core đã có `MeshResourceDescriptor` và
`insert_mesh_with_descriptor`. Descriptor này ghi nhận metadata tối thiểu của
vertex/index buffer và validate quan hệ giữa index buffer với index format.
Hai mesh trong fixture garbage-collection đã migrate sang API này; raw mesh
insertion không còn là phần bắt buộc của fixture đó.
