# IFOL GPU: texture descriptor validation

Validation structural cÃ²n bao gÃ¶m mip count khÃ´ng vÆ°á»£t quÃ¡ sá»‘ mip tá»‘i Ä‘a suy ra
tá»« extent vÃ  sample count pháº£i lÃ  lÅ©y thá»«a cá»§a hai. Giá»›i háº¡n sample count
thá»±c táº¿ theo format/adapter váº«n lÃ  capability runtime vÃ  pháº£i Ä‘Æ°á»£c
kiá»ƒm tra á»Ÿ device-aware validation; structural validation khÃ´ng tuyÃªn bá»‘
mọi sample count lÃ  backend-supported.

`TextureResourceDescriptor` là descriptor tối thiểu cho texture resource. Nó gồm
extent, layer, format, usage, mip count và sample count.

`validate(max_dimension)` từ chối extent bằng zero, layer/mip/sample bằng zero,
usage rỗng và kích thước vượt giới hạn device. `ResourceRegistry::insert_texture_with_descriptor`
chỉ mutation registry sau khi descriptor hợp lệ và giữ descriptor cho
validation/lifetime hiện tại.

Khi descriptor đã có trong registry, `RenderGraphExecutor::validate` đối chiếu
width/height của `RenderTarget::Offscreen` với descriptor và trả lỗi typed nếu lệch.
Validation cũng yêu cầu texture color/depth có usage `RENDER_ATTACHMENT` khi được
dùng làm attachment.
