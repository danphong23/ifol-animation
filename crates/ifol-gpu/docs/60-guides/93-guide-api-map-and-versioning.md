# API map và quy tắc sử dụng thư viện

Tài liệu này dành cho người tải crate về và cần biết đâu là public contract,
đâu là implementation detail.

## Đường dẫn import được khuyến nghị

```rust,ignore
use ifol_gpu::api::{GpuCapabilities, GpuError, RawTextureReadback};
use ifol_gpu::backend::{GpuEngine, GpuEngineBuilder};
use ifol_gpu::execution::{ExecutionReport, RenderGraphExecutor};
use ifol_gpu::graph::{ComputeCommand, CopyCommand, DrawCommand, RenderGraph, RenderNodePool};
use ifol_gpu::resources::{ResourceRegistry, TextureHandle};
```

Các module `backend`, `resources`, `graph`, `execution`, `memory`, `extensions`
và `api` là boundary public hiện tại. Không truy cập field `pub(super)`, raw map
nội bộ của registry, module test hoặc helper trong `tests/`.

## Quyền sở hữu

| Thành phần | Ai sở hữu | Quy tắc |
|---|---|---|
| `wgpu::Device`, `Queue` | `GpuEngine` | Lấy accessor, không tự thay thế bên trong engine |
| Texture/buffer/pipeline/bind group | Host + `ResourceRegistry` | Đăng ký descriptor trước khi graph dùng |
| Graph/node pool | Host | Graph giữ handle, không giữ GPU object |
| Submission completion | Host | Dùng `SubmissionTracker`/`FrameContext`/policy tương đương |
| File/media/color policy | Higher layer | Không đưa decoder/encoder vào core |

## Contract thực thi

`RenderGraphExecutor::execute_checked` tự validate, flatten và submit graph. Nó
trả `wgpu::SubmissionIndex`; host vẫn phải chờ completion trước khi tái sử dụng
hoặc giải phóng resource. Dùng `execute_checked_with_report` nếu cần thống kê
node/pass/usage cho diagnostics.

Nếu muốn validate trước khi submit, gọi `validate_with_device` bằng đúng engine
sẽ execute graph. Lỗi validation là typed `RenderGraphValidationError`;
extension không có dispatcher sẽ fail-closed bằng `UnsupportedExtension`.

## Surface và offscreen

- `RenderTarget::Screen` dùng surface view do host acquire và truyền vào
  `execute_with_surface_checked`.
- `RenderTarget::Offscreen` dùng texture owned/registered của host.
- `RenderTarget::OffscreenMsaa` dùng color resolve contract rõ ràng.

Resize/reconfigure surface do host điều phối event loop; engine chỉ cung cấp
`try_resize_surface` và `reconfigure_surface` với typed `SurfaceResizeError`.

## Error và capability

Luôn xử lý `Result`. Nhóm lỗi chính là `GpuError`, descriptor errors,
`RenderGraphValidationError`, `ReadbackError` và `Extension*Error`.
Không dùng fallback màu hoặc bỏ qua node để che lỗi contract. Host có thể log
`adapter_info`, `capabilities` và `ExecutionReport` để tạo diagnostics.

## Versioning và mức ổn định

Crate đang ở version `0.1.x`. Trong giai đoạn này public API có thể thay đổi
khi contract được chứng minh qua migration/test; module implementation và test
path không phải API ổn định. Mỗi thay đổi public phải cập nhật guide, example
hoặc migration note. Canonical export phải khóa input bytes, shader/graph
contract, raw readback và encoder ở higher layer.

Khi API đạt `1.0`, tài liệu này sẽ trở thành compatibility policy chính thức và
breaking change phải có migration guide.

## Checklist cho host mới

1. Khai báo dependency và đọc README crate.
2. Khởi tạo builder với backend/feature/limit phù hợp host.
3. Tạo resource bằng `wgpu`, đăng ký descriptor và giữ typed handle.
4. Tạo graph, command, usage và dependency.
5. Validate bằng đúng engine, execute bằng checked API.
6. Theo dõi submission trước khi reuse/destroy resource.
7. Dùng raw readback cho export; không dùng canvas preview làm source of truth.
8. Ghi adapter, format, raw hash và error report khi cần parity cross-platform.
