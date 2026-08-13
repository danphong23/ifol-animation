# IFOL GPU: audit trạng thái hiện tại

Tài liệu này là snapshot audit sau các task đã commit tới `01f4996`. Nó bổ
sung và ưu tiên hơn các ghi chú trạng thái cũ khi có mâu thuẫn.

## Đã có test gate

- graph phẳng, dependency tường minh và hazard tự động theo buffer range,
  texture mip/layer/aspect;
- render/compute/copy/indirect execution và validation trước submit;
- resource registry với descriptor metadata, version invalidation, owned
  texture và deferred destruction;
- `SubmissionTracker`, transient pools và `FrameContext`, gồm route owned
  texture vào deferred queue;
- surface resize/reconfigure, MSAA/resolve, depth/stencil và texture-copy
  aspect;
- async readback, execution report và timestamp profiling có submission gate;
- capability requirements, fallback adapter policy và backend probe Vulkan/GL/
  DX12/fallback trên host;
- dynamic bind-group limit, dynamic offset count/alignment và pipeline-layout
  signature metadata;
- bundle cache key theo resource version, attachment format, sample count và
  context key;
- compile check WebAssembly và Windows MSVC.

## Còn giới hạn, chưa được gọi là release candidate

- pipeline signature hiện do host cung cấp, chưa phải shader reflection đầy đủ;
- pass-level profiling, queue nhiều frame và present policy vẫn thuộc host;
- end-to-end fixture và visual parity trên từng backend chưa đủ;
- runtime matrix macOS/Metal, Linux, browser WebGPU, Android và iOS chưa chạy
  trong môi trường audit hiện tại;
- capability/format matrix chi tiết theo adapter chưa hoàn tất; core đã có
  snapshot và requirement validation, còn runtime evidence phải thu thập theo
  từng platform.

## Quy tắc kết luận

Compile target chỉ chứng minh Rust/API compile. Backend probe chỉ chứng minh
chính sách request adapter được thực thi. Không dùng hai loại evidence này để
tuyên bố runtime hoặc visual parity trên platform chưa chạy.

## Gate hiện tại

Gate host chuẩn là:

```text
cargo test -p ifol-gpu --lib --tests --examples -- --test-threads=1
cargo test -p ifol-gpu --bench render_benchmarks
cargo check -p ifol-gpu --target wasm32-unknown-unknown
cargo check -p ifol-gpu --target x86_64-pc-windows-msvc
```

Benchmark chạy riêng vì harness không nhận tham số `--test-threads` của test
runner. Test GPU phải chạy serialized do parallel all-targets từng gây heap
corruption trên host audit; đó là hạn chế của môi trường test, không được che
giấu bằng cách gọi pass giả.
