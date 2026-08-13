# Validation không panic

Các đường validation của graph phải trả `RenderGraphValidationError` cho input
thiếu hoặc metadata không nhất quán. Texture copy không dùng `unwrap`/`expect`
để giả định descriptor tồn tại; nếu ownership và descriptor registry lệch nhau,
compiler trả `MissingTextureDescriptor` trước khi encode.

Đây là boundary quan trọng giữa compatibility map của prototype và resource
model production. Khi map public còn tồn tại, host có thể tạo trạng thái không
đồng bộ bằng mutation trực tiếp; core phải báo lỗi có cấu trúc thay vì panic.
