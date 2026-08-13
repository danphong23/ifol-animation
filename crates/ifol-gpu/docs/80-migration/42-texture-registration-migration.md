# Texture registration qua registry API

Các example và benchmark đã chuyển texture view registration sang
`ResourceRegistry::insert_texture_with_descriptor`. Việc này đảm bảo
replacement đi qua validation và version tracking thay vì mutate map trực tiếp.

Các path cần copy/resolve/lifetime phải dùng `insert_owned_texture` cùng
descriptor đầy đủ; đây là khác biệt cố ý giữa view-only resource và owned
resource.

Raw texture insertion đã bị loại khỏi core; host phải chọn view-only descriptor
API hoặc owned texture API tùy yêu cầu lifetime.
