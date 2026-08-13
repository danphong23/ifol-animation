# Texture registration qua registry API

Các example và benchmark đã chuyển texture view registration sang
`ResourceRegistry::insert_texture`. Việc này đảm bảo replacement đi qua version
tracking thay vì mutate map trực tiếp.

API compatibility này chỉ lưu view/format. Các path cần copy/resolve/lifetime
phải dùng `insert_owned_texture` cùng descriptor đầy đủ; đây là khác biệt cố ý
giữa view-only resource và owned resource.

`ultimate_test_suite.rs` còn giữ mutation trực tiếp do đang có thay đổi prototype
riêng trong working tree; sẽ migrate ở task riêng để không trộn thay đổi người
dùng vào commit core.
