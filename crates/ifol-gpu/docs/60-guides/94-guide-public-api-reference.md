# Tra cứu public API

Đây là danh mục API hiện hành của crate `ifol-gpu`. Import path bên dưới
được kiểm tra theo source hiện tại; các module implementation nội bộ không phải
contract public.

## Khởi tạo backend

```rust,ignore
use ifol_gpu::backend::{GpuEngine, GpuEngineBuilder, GpuError};

let engine: GpuEngine<'_> = GpuEngineBuilder::new().build().await?;
println!("{:?}", engine.adapter_info());
```

`GpuEngineBuilder` hỗ trợ `with_backends`,
`with_power_preference`, `with_required_features`,
`with_required_limits`, `with_force_fallback_adapter` và
`with_surface`. `GpuEngine` cung cấp `device()`, `queue()`,
`capabilities()`, `adapter_info()`, `surface()`,
`try_resize_surface()`, `reconfigure_surface()` và
`surface_format()`.

Không có API `adapter()`; host dùng `adapter_info()` cho diagnostics.

## Resource registry

Các handle public nằm trong `ifol_gpu::resources`: `TextureHandle`,
`BufferHandle`, `PipelineHandle`, `ComputePipelineHandle`,
`BindGroupHandle`, `MeshHandle` và `RenderNodeId`.

`ResourceRegistry` nhận resource do host tạo cùng descriptor metadata. Các
API descriptor chính là:

- `insert_texture_with_descriptor`;
- `insert_owned_texture`;
- `insert_buffer_with_descriptor`;
- `insert_bind_group_with_descriptor`;
- `insert_pipeline_with_layout_descriptor`;
- `insert_compute_pipeline_with_layout_descriptor`;
- `insert_mesh_with_descriptor`.

Dùng owned texture khi cần copy, resolve, readback hoặc deferred destruction.
View-only texture chỉ phù hợp khi host tự giữ ownership và lifetime.

## Graph và execution

`RenderGraph`, `RenderNodePool`, `RenderTarget`, `DrawCommand`,
`ComputeCommand`, `CopyCommand` và `DrawAction` nằm trong module
`ifol_gpu::graph`. Graph khai báo target, node, dependency và resource usage;
host tạo shader/pipeline/bind group trước khi execute.

`RenderGraphExecutor` nằm trong `ifol_gpu::execution`:

- `validate` và `validate_with_device` chỉ kiểm tra;
- `execute_checked` trả `wgpu::SubmissionIndex`;
- `execute_checked_with_report` trả `ExecutionReport`;
- các biến thể `execute_with_surface_checked*` dùng target `Screen`;
- các biến thể timestamp/profiling chỉ thêm số đo, không thay đổi contract
  correctness.

Luôn xử lý `Result`. Validation failure không tự động chuyển thành
checkerboard hoặc một node khác.

## Readback

`RawTextureReadback` có các trường `bytes`, `width`, `height` và
`format`; bytes là row data không padding. Các API chính:

- `begin_texture_readback_checked` cho ticket bất đồng bộ;
- `read_texture_to_raw_with_format_checked` khi host truyền format rõ ràng;
- `read_texture_to_raw_from_registry_checked` cho owned texture lấy format từ
  descriptor trong registry.

Core không decode asset, đổi màu hay ghi PNG/JPEG/EXR/video. Host chịu trách
nhiệm canonical input, color/alpha policy và encoder output.

## Versioning và mức cam kết

Crate hiện ở `0.1.x`. Public module và kiểu lỗi ở trên là contract đang dùng,
nhưng chưa có cam kết tương thích như `1.0`. Khi đổi API, phải cập nhật guide,
example, test và migration note trong cùng thay đổi.
