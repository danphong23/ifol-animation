# IFOL GPU: copy pass buffer-to-buffer

`CopyCommand::buffer_to_buffer` mô tả copy giữa hai `BufferHandle`, có offset và
size. Registry có buffer namespace/version riêng. `RenderGraphExecutor` phát lệnh
`CommandEncoder::copy_buffer_to_buffer` trước compute/render pass.

Validation kiểm tra buffer tồn tại, offset+size không overflow và không vượt kích
thước buffer. Texture copy-to-texture và interleave scheduling đã có implementation
riêng; resolve/mipmap/aspect nâng cao vẫn là task sau.
