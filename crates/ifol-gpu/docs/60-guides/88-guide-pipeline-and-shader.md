# Hướng dẫn: pipeline và shader

`ifol-gpu` không biên dịch shader domain và không tự suy luận material. Host
tạo shader module, bind-group layout, pipeline layout và pipeline bằng `wgpu`.

```text
WGSL → bind group layout → pipeline layout → pipeline
    → insert_*_pipeline_with_layout_descriptor
```

Layout signature là metadata contract do host cung cấp. Reflection binding type,
visibility và min binding size chưa thuộc implementation hiện tại.

Command chỉ giữ `PipelineHandle`, bind groups và action. Graph không biết shader
đang làm blur, compositing, physics hay animation.

