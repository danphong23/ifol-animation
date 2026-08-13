# Hướng dẫn: xây dựng một graph

1. Chọn `RenderTarget`.
2. Đăng ký texture/buffer/pipeline/bind group ở host.
3. Tạo `RenderGraph` và thêm draw/compute/copy node.
4. Khai báo resource usage khi cần dependency rõ ràng.
5. Thêm explicit dependency khi quan hệ không suy ra từ usage.
6. Gọi `validate` hoặc `execute_checked`.

```text
UploadFrame → Filter → Composite → Readback
```

Compiler tạo hazard edges, flatten nested graph và tạo `FlatRenderPlan` trước
khi encode. Graph cũng dùng cho compute simulation, upload/copy, image
processing, particle, GPU culling, baking và offline frame generation.

