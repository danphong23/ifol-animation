# IFOL GPU: audit release baseline của core

Tài liệu này là checklist kết luận cho `ifol-gpu` core. Nó không tuyên bố
runtime parity trên các nền tảng mà môi trường audit chưa chạy.

## Checklist invariant core

| Hạng mục | Bằng chứng | Trạng thái |
|---|---|---|
| Graph dependency, hazard, cycle và flatten deterministic | Unit tests trong `graph` | Đạt |
| Render/compute/copy/indirect execution | Execution tests và typed validation | Đạt |
| Nested graph và flat execution plan | Flatten/segment tests | Đạt |
| Descriptor resource contract | Texture, buffer, pipeline, bind group, mesh tests | Đạt |
| Generational handle và version invalidation | Handle/registry/cache tests | Đạt |
| Submission-safe lifetime | Tracker, frame context, transient/deferred pool tests | Đạt |
| Surface resize/reconfigure | `SurfaceResizeError` tests và checked API | Đạt |
| Readback/save typed errors | Checked readback/save tests | Đạt |
| Extension registration/validation/dispatch | Extension registry/dispatcher tests | Đạt |
| Public API documentation | `cargo doc -p ifol-gpu --no-deps` | Đạt |

## Gate đã chạy trên host audit

```text
cargo test -p ifol-gpu --lib --tests --examples -- --test-threads=1
cargo test -p ifol-gpu --bench render_benchmarks
cargo check -p ifol-gpu --all-targets
cargo check -p ifol-gpu --target wasm32-unknown-unknown
cargo check -p ifol-gpu --target x86_64-pc-windows-msvc
cargo doc -p ifol-gpu --no-deps
```

Kết quả hiện tại: 104 unit tests, 1 integration test, toàn bộ example tests và
11 benchmark case pass. Các target WASM/MSVC compile pass; `cargo doc` pass.
GPU tests phải chạy serialized trên host này vì parallel execution từng gây
heap corruption ở môi trường kiểm thử.

## Không phải blocker của core

- Shader reflection, compiler, material và automatic pipeline-layout generation.
- Video/audio/editor/timeline, ECS, scene, animation và game-engine policy.
- Golden image, PSNR/SSIM và benchmark dashboard.
- Worker scheduling, present policy và nhiều pool profiling theo frame.

## Evidence còn thiếu để gọi là release candidate đa nền tảng

- Runtime/visual matrix trên Metal, Linux, browser WebGPU, Android và iOS.
- Capability/format snapshot thực tế của từng adapter mục tiêu.
- Surface lifecycle/presentation test trên từng nền tảng.

Các mục này cần CI, thiết bị hoặc browser harness tương ứng; không được suy ra
từ compile evidence trên Windows.
