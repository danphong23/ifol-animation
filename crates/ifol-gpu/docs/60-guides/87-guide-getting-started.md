# Hướng dẫn nhanh: dùng `ifol-gpu`

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

## Xây và chạy graph

```text
RenderGraph → add node/pass → declare usage/dependency → execute_checked
```

