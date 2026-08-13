# Khai báo resource usage trong graph

Graph hiện có metadata typed cho mỗi node:

```rust
graph.declare_resource_usage(
    node,
    GraphResource::Texture(texture),
    ResourceAccess::ReadWrite,
);
```

`GraphResource` hiện gồm buffer và texture; `ResourceAccess` gồm `Read`, `Write`
và `ReadWrite`. Compiler validation kiểm tra resource được khai báo có tồn tại
trong registry trước khi submit.

Đây là nền móng cho hazard analysis. Phiên bản hiện tại chưa tự sinh dependency
từ read/write declarations và chưa tạo backend barrier riêng; graph vẫn dùng
explicit dependency/flat ordering. Task tiếp theo sẽ dùng metadata này để phát
hiện read-after-write/write-after-read và bổ sung edges tự động.
