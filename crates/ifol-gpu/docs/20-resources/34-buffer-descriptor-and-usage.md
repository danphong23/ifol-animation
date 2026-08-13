# Buffer descriptor và usage validation

`wgpu::Buffer` không expose lại usage flags sau khi tạo. Vì vậy nếu graph cần
kiểm tra copy trước khi submit, host phải đăng ký buffer cùng
`BufferResourceDescriptor`:

```rust
BufferResourceDescriptor {
    size: 4096,
    usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
}
```

`insert_buffer_with_descriptor` kiểm tra size/usage và lưu metadata cạnh buffer.
Validation của `BufferToBuffer` dùng metadata này để yêu cầu source có
`COPY_SRC` và destination có `COPY_DST`, đồng thời vẫn kiểm tra range theo kích
thước buffer thật.

Raw `insert_buffer` đã bị loại khỏi core. Buffer phải đăng ký qua
`insert_buffer_with_descriptor` để graph nhận invariant usage và bounds đầy đủ.
