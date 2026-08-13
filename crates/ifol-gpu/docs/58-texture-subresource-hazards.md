# Texture subresource hazard model

`ResourceUsage` hiện có `subresource`:

- `Whole`: tương thích với API cũ, đại diện toàn resource;
- `Texture { mip_level, array_layer }`: mô tả một mip/layer cụ thể.

`declare_resource_usage` tiếp tục tạo usage `Whole`. Host cần độ chính xác cao
có thể dùng `declare_texture_subresource_usage`. Hai usage texture chỉ tạo
hazard khi cùng mip/layer hoặc một phía là `Whole`, và ít nhất một phía ghi.

Copy command tự động vẫn khai báo `Whole` ở bước này vì extent có thể bao phủ
nhiều layer; range đầy đủ cho copy và texture aspect sẽ là bước mở rộng tiếp
theo. Đây là cách giữ behavior an toàn: thiếu metadata không làm compiler bỏ
qua dependency cần thiết.
