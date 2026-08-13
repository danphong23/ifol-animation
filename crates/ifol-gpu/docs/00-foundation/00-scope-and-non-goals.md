# IFOL GPU: phạm vi và những điều không làm

## Phạm vi

`ifol-gpu` là thư viện GPU cấp thấp, không chứa semantic của game, phim,
animation, editor hay UI. Nó nhận resource, pipeline, command và graph; sau đó
validate, flatten, compile và submit qua `wgpu`.

Graph có thể phục vụ render 2D/2.5D/3D, compositing, compute, copy, simulation,
offline rendering và các phép tính GPU khác. Domain bên ngoài chịu trách nhiệm
scene, asset, animation, timeline, editor, audio, input và policy lưu file.

## Core chịu trách nhiệm

- graph dependency/hazard và flat execution plan;
- resource handle/descriptor/version/lifetime boundary;
- validation typed trước submit;
- capability requirements và fallback policy tường minh;
- surface/MSAA/resolve, readback và profiling primitive;
- memory reuse có submission completion gate.

## Core không làm

- không tự quản lý ECS, scene graph, material semantic hay animation timeline;
- không tự chọn fallback domain-specific khi asset thiếu;
- không hứa visual parity/runtime parity trên platform chưa có evidence;
- không tự present, poll event loop, map readback theo chính sách ứng dụng;
- không tự suy luận shader reflection khi host chưa cung cấp metadata.

Trạng thái implementation và giới hạn xem [audit hiện tại](../70-status/80-current-audit.md).
