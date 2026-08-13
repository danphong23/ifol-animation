# Texture subresource hazard model

`ResourceUsage` hiện có `subresource`:

- `Whole`: tương thích với API cũ, đại diện toàn resource;
- `BufferRange { start..end }`: mô tả vùng byte của buffer;
- `Texture { mip_level, array_layer }`: mô tả một mip/layer cụ thể;
- `TextureRange { mip_start..mip_end, layer_start..layer_end }`: mô tả range
  mip/layer, dùng cho texture copy.
- `TextureAspect`/`TextureAspectRange`: thêm `All`, `DepthOnly` hoặc
  `StencilOnly` cho depth-stencil resource.

`declare_resource_usage` tiếp tục tạo usage `Whole`. Host cần độ chính xác cao
có thể dùng `declare_texture_subresource_usage`. Hai usage texture chỉ tạo
hazard khi cùng mip/layer hoặc một phía là `Whole`, và ít nhất một phía ghi.

Với buffer, dùng `declare_buffer_range_usage`; hai range chỉ xung đột khi
chúng overlap. Range rỗng hoặc phép cộng offset bị overflow sẽ fallback về
`Whole` để giữ an toàn.

Texture copy tự động suy ra range từ mip, origin và extent. Nếu phép tính range
bị overflow, compiler fallback về `Whole` để không bỏ qua dependency cần thiết.
Buffer copy tự động suy ra `BufferRange` từ offset/size. Render attachment vẫn
dùng `Whole` để giữ dependency bảo thủ; host có thể khai báo depth/stencil aspect
riêng bằng `declare_texture_aspect_usage` hoặc API range tương ứng.
