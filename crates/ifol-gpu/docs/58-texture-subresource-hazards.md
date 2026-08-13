# Texture subresource hazard model

`ResourceUsage` hiện có `subresource`:

- `Whole`: tương thích với API cũ, đại diện toàn resource;
- `Texture { mip_level, array_layer }`: mô tả một mip/layer cụ thể;
- `TextureRange { mip_start..mip_end, layer_start..layer_end }`: mô tả range
  mip/layer, dùng cho texture copy.

`declare_resource_usage` tiếp tục tạo usage `Whole`. Host cần độ chính xác cao
có thể dùng `declare_texture_subresource_usage`. Hai usage texture chỉ tạo
hazard khi cùng mip/layer hoặc một phía là `Whole`, và ít nhất một phía ghi.

Texture copy tự động suy ra range từ mip, origin và extent. Nếu phép tính range
bị overflow, compiler fallback về `Whole` để không bỏ qua dependency cần thiết.
Buffer copy và render attachment vẫn dùng `Whole` vì chưa có byte-range/aspect
model tương ứng.
