# IFOL GPU: compute pass hiện tại

`RenderGraph::add_compute_batch` thêm một `ComputeBatch` vào logical graph. Mỗi
`ComputeCommand` khai báo compute pipeline, bind groups/dynamic offsets và số
workgroup `[x, y, z]`.

`RenderGraphExecutor` tạo `wgpu::ComputePass` và dispatch compute nodes theo flat
execution order. Graph mixed render/compute dùng ordered segments để không gom
compute ra khỏi vị trí logical của node. Validation kiểm tra compute pipeline,
bind group và slot.

Đây là bước đầu để dùng ifol-gpu cho simulation/data processing. Storage resource
usage, dispatch limit và dependency hazard tổng quát vẫn cần pass model đầy đủ
hơn; compute/render interleave cơ bản đã được implement.
