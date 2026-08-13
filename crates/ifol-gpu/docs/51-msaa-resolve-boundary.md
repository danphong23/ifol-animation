# MSAA và resolve boundary

Render compiler hiện hỗ trợ target `RenderTarget::OffscreenMsaa`: color
attachment multisample được resolve vào texture single-sample trong cùng render
pass. `Offscreen` cũ vẫn giữ sample count bằng 1.

Validation bắt buộc:

- color attachment có sample count lớn hơn 1;
- resolve target có sample count bằng 1;
- format và kích thước của hai texture giống nhau;
- cả hai texture có usage `RENDER_ATTACHMENT`.

Depth/stencil MSAA, subresource/aspect và capability-limit theo adapter vẫn là
phần tiếp theo; target sample count không được coi là đã chạy trên mọi backend
chỉ vì validation graph thành công.
