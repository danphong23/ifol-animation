# IFOL GPU: bằng chứng compile đa nền tảng

## Nguyên tắc

Compile thành công chứng minh crate không có lỗi target-specific ở tầng Rust/API
đã được kiểm tra. Nó không chứng minh adapter runtime, surface lifecycle,
shader support hay visual parity trên nền tảng đó.

## Evidence hiện tại

Môi trường audit ngày 2026-08-13:

| Target | Lệnh | Kết quả | Phạm vi |
|---|---|---|---|
| Host Windows GNU | `cargo test -p ifol-gpu --all-targets` | Đạt | 75 unit tests, integration, benchmarks, examples |
| WebAssembly | `cargo check -p ifol-gpu --target wasm32-unknown-unknown` | Đạt | compile crate, chưa chạy browser |
| Windows MSVC | `cargo check -p ifol-gpu --target x86_64-pc-windows-msvc` | Đạt | compile crate, chưa chạy MSVC runtime |

## Còn thiếu

- runtime adapter/backend matrix trên Vulkan, Metal, DX12, GLES/WebGPU;
- browser harness cho WebGPU;
- lifecycle/surface test trên Android và iOS;
- visual snapshot và readback parity từng backend.

Vì vậy Definition of Done vẫn giữ cross-platform runtime ở trạng thái chưa đạt.
Không được dùng compile evidence này để tuyên bố mọi feature có trên mọi thiết bị.

