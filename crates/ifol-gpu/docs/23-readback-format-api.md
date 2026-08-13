# IFOL GPU: readback theo texture format

Host phải truyền `TextureFormat` thật cho readback vì `wgpu::Texture` không
expose descriptor đầy đủ sau khi tạo. Core bỏ row padding và trả raw bytes cùng
width/height.

API legacy `read_texture_to_bytes` vẫn giả định `Rgba8UnormSrgb`. API mới nên
dùng `read_texture_to_bytes_with_format_checked` hoặc
`begin_texture_readback_checked`; API checked trả `ReadbackError` typed cho
format không hỗ trợ, extent lỗi, overflow, map failure hoặc access failure.

Core chưa chuyển đổi depth/stencil/compressed format và không tự đoán format.
Chi tiết xem [typed readback errors](81-typed-readback-errors.md).
