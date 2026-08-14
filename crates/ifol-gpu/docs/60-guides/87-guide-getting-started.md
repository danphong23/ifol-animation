# 1. Bắt đầu nhanh với `ifol-gpu`

Đây là tài liệu đầu tiên trong [public usage guide](README.md). Sau trang này,
đọc [đăng ký resource](91-guide-resource-registration.md).

## Khởi tạo engine

```rust
let engine = GpuEngineBuilder::new().build().await?;
let capabilities = engine.capabilities();
```

Host chọn backend, required features, limits và fallback policy. Core không tự
chọn policy domain-specific.

## Đăng ký resource

Host tạo `wgpu::Texture`, `wgpu::Buffer`, pipeline và bind group; sau đó đăng ký
bằng API có descriptor. Descriptor phải mô tả mọi thuộc tính ảnh hưởng đến
validation và cache.

```text
create wgpu resource → insert_*_with_descriptor → giữ handle → graph tham chiếu
```

Các API resource chính là:

- texture view: `insert_texture_with_descriptor`;
- texture có ownership/lifetime cho copy hoặc resolve: `insert_owned_texture`;
- buffer: `insert_buffer_with_descriptor`;
- render/compute pipeline: API `*_with_layout_descriptor` tương ứng;
- bind group: `insert_bind_group_with_descriptor`;
- mesh: `insert_mesh_with_descriptor`.

Raw insertion không còn là API của core; mọi resource phải đi kèm metadata cần
cho validation và cache invalidation.

## Xây và chạy graph

```text
RenderGraph → add node/pass → declare usage/dependency → execute_checked
```

Graph không chứa shader semantic hoặc dữ liệu domain; nó chỉ giữ command, handle
và usage để core sắp xếp, validate rồi execute.
