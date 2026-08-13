# Pipeline mutation qua registry API

Các example và benchmark đã chuyển đăng ký pipeline sang
`registry.insert_pipeline_with_layout_descriptor(...)`. API này luôn tăng
resource version và lưu metadata layout, nên bundle/cache có thể nhận biết
pipeline bị thay thế mà không cần raw insertion.

Pipeline raw insertion đã bị loại khỏi core; host phải cung cấp
`PipelineLayoutResourceDescriptor` khi đăng ký pipeline.
