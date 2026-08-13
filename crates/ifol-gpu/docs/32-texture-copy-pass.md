# Texture copy pass

`CopyCommand` hỗ trợ:

- `BufferToBuffer`: copy một vùng byte giữa hai buffer;
- `TextureToTexture`: copy vùng texel với aspect `All`;
- `TextureToTextureAspect`: copy `All`, `DepthOnly` hoặc `StencilOnly` khi format hỗ trợ.

Texture nguồn và đích phải được đăng ký bằng `insert_owned_texture`, có cùng
format, source có `COPY_SRC` và destination có `COPY_DST`. Compiler kiểm tra
ownership, mip, extent, bounds, format, usage và aspect trước khi encode.

`with_texture_mips` chọn mip level source/destination. Resolve MSAA, chuyển format
và copy giữa dimension khác nhau không thuộc API này.

Runtime test xác nhận texture RGBA 2D được copy và đọc lại đúng qua readback API.

