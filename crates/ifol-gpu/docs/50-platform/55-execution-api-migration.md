# Migration execution API có `Result`

`RenderGraphExecutor::execute_checked` và
`execute_with_surface_checked` trả:

```text
Result<wgpu::SubmissionIndex, RenderGraphValidationError>
```

Host production phải xử lý lỗi hoặc chuyển tiếp lỗi; không có API public nào
encode một graph invalid bằng cách bỏ qua resource thiếu một cách âm thầm.
Encoder unchecked chỉ còn là implementation detail sau khi validation đã pass.

Các benchmark/example nội bộ dùng `expect` với message cụ thể vì graph của
chúng là fixture phải hợp lệ. Ứng dụng thật nên dùng `match`, `?` hoặc log lỗi
theo policy của host.
