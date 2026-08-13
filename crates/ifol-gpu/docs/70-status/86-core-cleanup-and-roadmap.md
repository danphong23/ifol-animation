# IFOL GPU: boundary, cleanup và roadmap hoàn thiện core

Operation contract đã bổ sung khai báo `ResourceUsage` và validation range;
dispatch vào node/flat plan/executor vẫn là task riêng.

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

- source layout chưa phản ánh ba tầng;
- compatibility insert API và alias `TextureCache` còn tồn tại;
- private encoder còn silent skip ở một số nhánh;
- docs migration/status còn stale;
- pipeline layout mới là host metadata, chưa có shader reflection;
- capability/format matrix và runtime matrix đa platform chưa đủ evidence;
- extension boundary cho custom graph operation chưa hoàn chỉnh.

## Roadmap bắt buộc

1. Đóng băng graph kernel public contract.
2. Tách source tree theo boundary ở trên, giữ re-export tạm thời.
3. Migrate examples/tests sang descriptor API và xóa compatibility path.
4. Loại bỏ silent skip trong encoder hoặc biến thành typed internal error.
5. Chuẩn hóa built-in operation và custom extension boundary.
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
test duplicate/empty ID. Custom operation vẫn chưa được tích hợp vào node, flat
plan hoặc executor.
`Extension` node đã có representation trong graph và được flatten theo usage;
executor hiện chủ động trả `UnsupportedExtension` cho node chưa có dispatch.
`Extension` node đã có representation trong graph và được flatten theo usage;
executor hiện chủ động trả `UnsupportedExtension` cho node chưa có dispatch.
