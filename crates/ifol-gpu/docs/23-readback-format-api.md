# IFOL GPU: readback theo texture format

`GpuEngine::read_texture_to_bytes_with_format` yêu cầu host truyền
`TextureFormat` thật của texture. API trả raw bytes theo row đã bỏ padding và kích
thước width/height.

API `read_texture_to_bytes` cũ vẫn tồn tại cho compatibility và giả định
`Rgba8UnormSrgb`; host mới nên dùng API có format. Depth/stencil và format chưa có
bytes-per-pixel mapping sẽ trả lỗi thay vì đọc sai dữ liệu.
