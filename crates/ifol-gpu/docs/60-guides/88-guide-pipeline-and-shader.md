# 3. Pipeline và shader

Đây là phần thứ ba của [public usage guide](README.md). Resource registration
được mô tả ở [trang trước](91-guide-resource-registration.md).

`ifol-gpu` không biên dịch shader domain và không tự suy luận material. Host
tạo shader module, bind-group layout, pipeline layout và pipeline bằng `wgpu`.

```text
WGSL → bind group layout → pipeline layout → pipeline
    → insert_pipeline_with_layout_descriptor
      hoặc insert_compute_pipeline_with_layout_descriptor
```

Layout signature là metadata contract do host cung cấp. Reflection binding type,
visibility và min binding size chưa thuộc implementation hiện tại.

## API đăng ký tương ứng

| Resource | API bắt buộc | Metadata chính |
|---|---|---|
| Render pipeline | `insert_pipeline_with_layout_descriptor` | bind-group layout signatures |
| Compute pipeline | `insert_compute_pipeline_with_layout_descriptor` | bind-group layout signatures |
| Bind group | `insert_bind_group_with_descriptor` | dynamic offsets, alignment, layout signature |

Core không nhận shader source và không tạo pipeline thay host. Host chịu trách
nhiệm bảo đảm shader entry point, vertex layout, bind-group layout và target
format tương thích với pipeline descriptor đã đăng ký.

Command chỉ giữ `PipelineHandle`, bind groups và action. Graph không biết shader
đang làm blur, compositing, physics hay animation.
