# IFOL GPU: fallback adapter policy

`GpuEngineBuilder` mặc định chọn adapter phần cứng (`force_fallback_adapter =
false`). Host có thể chọn software/fallback adapter bằng:

```rust
GpuEngineBuilder::new()
    .with_force_fallback_adapter(true)
    .build()
    .await?;
```

Policy này được truyền trực tiếp vào `wgpu::RequestAdapterOptions`; core không tự
âm thầm chuyển sang fallback khi adapter phần cứng không đáp ứng capability.
Host phải chọn policy phù hợp với use case: headless/CI có thể ưu tiên fallback,
còn game/render realtime thường yêu cầu adapter phần cứng.

Không phải nền tảng nào cũng có software adapter khả dụng. Khi không có adapter,
builder trả `NoAdapterFound`/`AdapterRequestFailed`; đây là kết quả capability có
thể xử lý, không phải panic.

