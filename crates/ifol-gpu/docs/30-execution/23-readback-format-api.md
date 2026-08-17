# IFOL GPU: readback theo texture format

Với texture truyền trực tiếp, host phải truyền `TextureFormat` thật vì
`wgpu::Texture` không expose descriptor đầy đủ sau khi tạo. Với texture owned
trong `ResourceRegistry`, host có thể dùng API registry để core lấy format từ
descriptor đã đăng ký. Core bỏ row padding và trả raw bytes cùng width/height.

Host dùng `read_texture_to_raw_with_format_checked`,
`read_texture_to_raw_from_registry_checked` hoặc
`begin_texture_readback_checked`; API trả raw bytes kèm `format` và
`ReadbackError` typed cho
format không hỗ trợ, extent lỗi, overflow, map failure hoặc access failure.

API registry chỉ áp dụng cho texture owned vì registry phải giữ backing
`wgpu::Texture` để thực hiện copy. Texture view-only vẫn phải dùng API
explicit với texture và format do host quản lý.

Core chưa chuyển đổi depth/stencil/compressed format và không tự đoán color
policy. `RenderTarget::Offscreen` và `RenderTarget::OffscreenMsaa` là đường
canonical để tạo output độc lập với surface của desktop/web/mobile.
Chi tiết xem [typed readback errors](../70-status/81-typed-readback-errors.md).
