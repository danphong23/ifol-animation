# Phân loại phạm vi: core, GPU infrastructure và engine bên ngoài

Tài liệu này là quyết định phạm vi hiện hành sau khi audit commit history. Nó
giúp phân biệt việc cần hoàn thiện trong `ifol-gpu` với tính năng của engine sử
dụng `ifol-gpu`.

## Core bắt buộc

Các phần sau là invariant hoặc execution contract của GPU work graph:

- graph, node, resource usage, dependency, hazard và cycle detection;
- flatten nested graph thành flat execution plan deterministic;
- render, compute, copy và indirect command boundary;
- typed validation trước submit và fail-closed error path;
- generational handle, resource registry, descriptor và version;
- submission-safe lifetime, deferred destruction và transient reuse;
- backend-neutral device/capability/surface boundary;
- extension identity, operation usage, validation và dispatch contract.

Graph kernel không được biết video, game scene, ECS, animation, material,
timeline, codec hay semantic của shader.

## GPU infrastructure trong crate nhưng ngoài graph kernel

Các phần này hợp lý trong `ifol-gpu`, thường có thể là module/feature optional:

- device/queue/adapter và capability snapshot;
- pipeline/bind-group validation và cache;
- memory/frame allocator, submission tracker;
- readback typed và surface lifecycle;
- profiling hook, execution report và diagnostics;
- backend/runtime capability matrix.

Chúng phục vụ việc encode graph an toàn, nhưng không được làm graph model biết
chi tiết backend hoặc domain application.

## Engine/tool bên ngoài

Các phần sau không phải điều kiện hoàn thiện core:

- shader compiler, shader reflection và automatic pipeline-layout generation;
- material, scene, ECS, animation, physics và game engine;
- video decode/encode, codec, audio, timeline và video editor;
- asset database, hot-reload policy và editor UI;
- PSNR/SSIM, golden-image harness và benchmark dashboard.

Host có thể tự tạo `PipelineLayoutDescriptor`, hoặc dùng một tool reflection để
sinh descriptor đó. `ifol-gpu` chỉ validate descriptor đã chuẩn hóa; core không
cần parse shader source.

## Quy tắc test theo boundary

| Mục tiêu | Nơi kiểm tra |
|---|---|
| Graph order, hazard, cycle, flat plan | Unit/integration test của core |
| GPU command và resource lifetime | Runtime test của crate trên backend khả dụng |
| Pixel/ảnh render đúng | Readback + visual test harness, không phải graph kernel |
| Shader reflection/material/video/editor | Test của tool hoặc engine bên ngoài |
| Throughput/latency/memory | Benchmark harness, không phải correctness gate |

## Task còn lại sau khi phân loại

### Bắt buộc cho core

1. Mở rộng test để chứng minh extension usage đi cùng validation/lifetime path
   với built-in node.
2. Loại bỏ silent skip trong mọi public execution path.
3. Migrate consumer nội bộ khỏi raw registry API và compatibility facade.
4. Audit typed errors, deterministic diagnostics và nhiều frame in-flight.
5. Full correctness/lifetime audit và release checklist.

### GPU infrastructure cần evidence trước release candidate

8. Capability/format/runtime matrix trên các backend khả dụng.
9. Profiling/readback đa frame và benchmark tách compile/encode/submit/GPU.
10. Surface/presentation boundary audit.

Reflection, golden image, PSNR/SSIM và các engine feature không được thêm vào
core task plan trừ khi có yêu cầu mới rõ ràng.
