# IFOL GPU: descriptor-aware dynamic offset validation

## Giới hạn introspection

`wgpu::BindGroup` không expose bind-group layout để core tự suy luận số lượng
dynamic offset hoặc loại buffer tương ứng. Compatibility API cũ vì vậy vẫn cho
phép đăng ký bind group không có metadata, nhưng không thể kiểm tra sâu hơn
slot tồn tại.

## Contract metadata

Host có thể dùng `insert_bind_group_with_descriptor` với:

```text
BindGroupResourceDescriptor {
    dynamic_offset_count,
    dynamic_offset_alignment,
}
```

Khi metadata có mặt, graph validation kiểm tra:

- số offset trong mỗi command đúng với số binding dynamic;
- mọi offset chia hết cho alignment;
- alignment khác zero và là lũy thừa của hai.

Validation xảy ra trước command buffer submission và trả typed error. Dynamic
offset không bị bake sai vào bundle: bundle key vẫn chứa vector offset hiện tại.

## Phạm vi còn lại

Metadata này là contract do host cung cấp, chưa thay thế pipeline-layout
compatibility đầy đủ. Task tiếp theo phải bổ sung layout signature hoặc resource
reflection nếu muốn kiểm tra bind-group layout với pipeline một cách độc lập.

## Test gate

Có test descriptor shape, test graph với dynamic uniform binding hợp lệ và test
offset lệch alignment bị từ chối.
