# 4. Xây dựng một graph

Đây là phần thứ tư của [public usage guide](README.md). Sau graph, đọc
[execute và lifecycle](92-guide-execution-and-lifecycle.md).

1. Chọn `RenderTarget` hoặc graph không có render target nếu chỉ compute/copy.
2. Đăng ký texture/buffer/pipeline/bind group ở host.
3. Tạo `RenderGraph` và thêm draw/compute/copy node.
4. Khai báo resource usage khi cần dependency rõ ràng.
5. Thêm explicit dependency khi quan hệ không suy ra từ usage.
6. Gọi `validate` hoặc `execute_checked`; chỉ submit sau khi validation pass.

```text
UploadFrame → Filter → Composite → Readback
```

Compiler tạo hazard edges, flatten nested graph và tạo `FlatRenderPlan` trước
khi encode. Graph cũng dùng cho compute simulation, upload/copy, image
processing, particle, GPU culling, baking và offline frame generation.

### Quy tắc ownership

Graph chỉ giữ handle và mô tả usage; graph không sở hữu `wgpu::Texture`,
`wgpu::Buffer` hay pipeline. Registry/host quản lý lifetime resource. Khi cần
copy, resolve hoặc deferred destruction, resource phải được đăng ký bằng API
owned tương ứng và được giải phóng theo submission completion.
