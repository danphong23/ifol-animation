# IFOL GPU: texture descriptor validation

`TextureResourceDescriptor` là descriptor tối thiểu cho texture resource. Nó gồm
extent, layer, format, usage, mip count và sample count.

`validate(max_dimension)` từ chối extent bằng zero, layer/mip/sample bằng zero,
usage rỗng và kích thước vượt giới hạn device. `ResourceRegistry::insert_texture_with_descriptor`
chỉ mutation registry sau khi descriptor hợp lệ và giữ descriptor để compiler có
thể dùng cho compatibility/lifetime validation về sau.

API texture insert cũ vẫn tồn tại cho prototype compatibility; production path nên
dùng API có descriptor.

Khi descriptor đã có trong registry, `RenderGraphExecutor::validate` đối chiếu
width/height của `RenderTarget::Offscreen` với descriptor và trả lỗi typed nếu lệch.
Validation cũng yêu cầu texture color/depth có usage `RENDER_ATTACHMENT` khi được
dùng làm attachment.
