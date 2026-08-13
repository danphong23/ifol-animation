# IFOL GPU: copy pass buffer-to-buffer

`CopyCommand::buffer_to_buffer` mô tả copy giữa hai `BufferHandle`, có offset và
size. Registry có buffer namespace/version riêng. `RenderGraphExecutor` phát lệnh
`CommandEncoder::copy_buffer_to_buffer` trước compute/render pass.

Validation kiểm tra buffer tồn tại, offset+size không overflow và không vượt kích
thước buffer. Texture copy, resolve/mipmap và interleave scheduling vẫn là task sau.
