# IFOL GPU: readback theo texture format

Host phải truyền `TextureFormat` thật cho readback vì `wgpu::Texture` không
expose descriptor đầy đủ sau khi tạo. Core bỏ row padding và trả raw bytes cùng
width/height.

Host dùng `read_texture_to_raw_with_format_checked` hoặc
`begin_texture_readback_checked`; API trả raw bytes kèm `format` và
`ReadbackError` typed cho
format không hỗ trợ, extent lỗi, overflow, map failure hoặc access failure.

Core chưa chuyển đổi depth/stencil/compressed format và không tự đoán format.
Chi tiết xem [typed readback errors](../70-status/81-typed-readback-errors.md).
