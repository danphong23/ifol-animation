# IFOL GPU: texture descriptor validation

`TextureResourceDescriptor` là descriptor tối thiểu cho texture resource. Nó gồm
extent, layer, format, usage, mip count và sample count.

`validate(max_dimension)` từ chối extent bằng zero, layer/mip/sample bằng zero,
usage rỗng và kích thước vượt giới hạn device. `ResourceRegistry::insert_texture_with_descriptor`
chỉ mutation registry sau khi descriptor hợp lệ và giữ descriptor để compiler có
thể dùng cho compatibility/lifetime validation về sau.

API texture insert cũ vẫn tồn tại cho prototype compatibility; production path nên
dùng API có descriptor.
