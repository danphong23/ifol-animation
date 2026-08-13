# IFOL GPU: Mô hình resource

## Các nhóm resource

Mô hình mục tiêu gồm các resource typed riêng biệt:

- texture và texture view;
- buffer;
- sampler;
- shader module;
- bind group layout và bind group;
- render pipeline và compute pipeline;
- vertex/index stream;
- render target và depth/stencil attachment.

`ResourceRegistry` hiện tại là prototype và expose các `HashMap` public. API production nên cung cấp insert, lookup, replace và destroy có kiểm soát để version của resource có thể invalidate compiled artifact an toàn.

## Handle

Numeric handle phù hợp cho command compact, nhưng `u64` đơn thuần không đủ cho resource store sống lâu. Handle mục tiêu là handle có generation:

```text
Handle { index: u32, generation: u32 }
```

Handle cũ hoặc sai loại phải trả về lỗi typed, không được âm thầm trỏ sang resource mới.

## Resource descriptor

Descriptor phải chứa mọi thuộc tính ảnh hưởng đến compatibility và reuse. Với texture, tối thiểu gồm dimension, extent, format, usage, mip count, array layer, sample count và view compatibility. Kích thước bị lặp trong graph target phải được validate với texture thật hoặc bỏ khỏi target description.

## Ownership và destruction

Destruction được gọi rõ ràng ở API, nhưng việc thu hồi vật lý có thể trì hoãn tới khi GPU hoàn thành mọi submission đang tham chiếu resource. Resource manager phải theo dõi submission safety; Rust wrapper bị drop không có nghĩa GPU đã dùng xong.

## Binding

Bind group có thể do host tạo hoặc tạo qua resource API. Core không suy luận semantic group như global/material/entity; đó là convention của tầng trên. Core phải validate slot, dynamic offset alignment, resource usage và pipeline layout compatibility.

## Tên gọi texture cache

`TextureCache` hiện tại là exact-descriptor free-list, không phải LRU và không phải hệ thống eviction VRAM. Nó cần đổi tên thành `TransientTexturePool` hoặc được thiết kế lại trước khi gọi là LRU. GPU memory pressure và asset eviction thuộc resource manager có ownership/usage tracking rõ ràng.
