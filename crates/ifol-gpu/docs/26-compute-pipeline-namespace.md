# IFOL GPU: compute pipeline namespace

Compute pipeline dùng `ComputePipelineHandle` riêng, không dùng chung
`PipelineHandle` của render pipeline. `ResourceRegistry` lưu compute pipeline và
version độc lập để compiled artifact của render/compute không invalidate nhầm nhau.

Đây mới là resource/API foundation. Compute command, storage usage validation và
compute pass execution sẽ được triển khai ở task tiếp theo.
