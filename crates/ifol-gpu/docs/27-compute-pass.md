# IFOL GPU: compute pass hiện tại

`RenderGraph::add_compute_batch` thêm một `ComputeBatch` vào logical graph. Mỗi
`ComputeCommand` khai báo compute pipeline, bind groups/dynamic offsets và số
workgroup `[x, y, z]`.

`RenderGraphExecutor` tạo `wgpu::ComputePass` và dispatch compute nodes trước render
pass của graph hiện tại. Validation kiểm tra compute pipeline, bind group và slot.

Đây là bước đầu để dùng ifol-gpu cho simulation/data processing. Storage resource
usage, dispatch limit, compute/render interleave và dependency hazard vẫn cần compiler
pass model đầy đủ hơn.
