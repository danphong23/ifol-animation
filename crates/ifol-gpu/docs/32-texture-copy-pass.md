# Texture copy pass

`CopyCommand` hỗ trợ hai loại phép copy độc lập:

- `BufferToBuffer`: copy một vùng byte giữa hai buffer.
- `TextureToTexture`: copy một vùng texel giữa hai texture cùng format.

## Điều kiện bắt buộc

Texture source và destination phải được đăng ký bằng `insert_owned_texture`, vì
compiler cần giữ `wgpu::Texture` thật chứ không chỉ giữ `TextureView`. Descriptor
của hai texture phải có format giống nhau; source cần `COPY_SRC`, destination
cần `COPY_DST`.

Compiler kiểm tra trước khi encode:

- texture có tồn tại và có ownership;
- mip level hợp lệ;
- extent không có chiều bằng zero;
- origin cộng extent không vượt quá kích thước mip tương ứng;
- format và usage tương thích.

## API và giới hạn hiện tại

```rust
CopyCommand::texture_to_texture(source, destination, [width, height, layers])
```

`with_texture_mips` chọn mip level source/destination. Phiên bản hiện tại dùng
`TextureAspect::All`, phù hợp cho texture màu thông thường. Resolve MSAA,
depth/stencil aspect, chuyển format và copy giữa dimension khác nhau chưa thuộc
API này; chúng sẽ là command riêng khi graph model có explicit subresource/aspect.

Runtime test đã xác nhận dữ liệu RGBA 2D được copy và đọc lại đúng bằng API
readback theo format.
