# MSAA và resolve boundary

Render compiler hiện hỗ trợ target `RenderTarget::OffscreenMsaa`: color
attachment multisample được resolve vào texture single-sample trong cùng render
pass. `Offscreen` cũ vẫn giữ sample count bằng 1.

Validation bắt buộc:

- color attachment có sample count lớn hơn 1;
- resolve target có sample count bằng 1;
- depth attachment (nếu có) có cùng sample count với color attachment;
- format và kích thước của hai texture giống nhau;
- cả hai texture có usage `RENDER_ATTACHMENT`.

Depth MSAA cơ bản và stencil aspect (`Stencil8`, `Depth24PlusStencil8`,
`Depth32FloatStencil8`) đã được nối vào render pass và có runtime test. Việc
chọn sample count cuối cùng vẫn do device/backend xác nhận khi tạo texture;
`wgpu::Limits` không cung cấp một `max_sample_count` chung để core tự suy diễn.
Subresource model và capability matrix đa backend vẫn là phần tiếp theo.
