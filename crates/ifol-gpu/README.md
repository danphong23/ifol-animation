# `ifol-gpu`

`ifol-gpu` là GPU execution core của iFol. Crate nhận resource, pipeline,
shader contract và render graph từ host; sau đó validate, sắp xếp hazard, thực
thi và có thể trả raw readback.

Crate này không phải image/video pipeline và không tự biết asset là PNG, JPEG,
WebP hay video. Decoder, color/alpha policy, canonical asset bytes và encoder
thuộc tầng ngoài.

## Trạng thái API

API public hiện ở mức `0.1.x`: dùng được như thư viện phát triển nội bộ và host
engine, nhưng contract chưa phải cam kết ổn định `1.0`.

| Nhu cầu | Module/API |
|---|---|
| Khởi tạo device/queue | `backend::GpuEngineBuilder`, `backend::GpuEngine` |
| Capability/error | `api::{GpuCapabilities, GpuError, CapabilityError}` |
| Resource registry/handles | `resources::{ResourceRegistry, *Handle, *Descriptor}` |
| Graph/pass/command | `graph::{RenderGraph, RenderNodePool, DrawCommand, ComputeCommand, CopyCommand}` |
| Validate/execute | `execution::{RenderGraphExecutor, ExecutionReport}` |
| Raw readback | `api::{RawTextureReadback, ReadbackError, ReadbackTicket}` |
| Lifetime/profiling | `memory::*`, `api::profiling` |
| Custom graph extension | `extensions::{ExtensionDispatchRegistry, GpuExtension}` |

## Cài đặt trong workspace

```toml
[dependencies]
ifol-gpu = { path = "../crates/ifol-gpu" }
```

Crate hiện được phát triển trong workspace; chưa giả định đã phát hành trên
crates.io.

## Khởi tạo tối thiểu

```rust,ignore
use ifol_gpu::backend::{GpuEngineBuilder, GpuError};

async fn create_gpu() -> Result<(), GpuError> {
    let engine = GpuEngineBuilder::new().build().await?;
    println!("adapter: {:?}", engine.adapter_info());
    Ok(())
}
```

Host chọn async runtime, backend, feature và limit. Headless/offscreen host
không cần `winit`; window host tự tạo `wgpu::Surface` rồi truyền vào builder.
`ifol-gpu` không sở hữu event loop.

## Luồng sử dụng chuẩn

```text
host tạo wgpu resource
        ↓
ResourceRegistry::insert_*_with_descriptor
        ↓
host tạo RenderGraph + RenderNodePool + commands
        ↓
RenderGraphExecutor::validate_with_device
        ↓
RenderGraphExecutor::execute_checked
        ↓
SubmissionIndex / ExecutionReport
        ↓
host quản lý completion và lifetime
        ↓
GpuEngine::read_texture_to_raw_* (nếu cần readback)
```

Graph chỉ giữ typed handle và usage contract; graph không sở hữu GPU object.
Resource phải sống đến sau submission cuối cùng sử dụng nó.

## Resource và pipeline contract

Host tự tạo `wgpu::Texture`, `wgpu::Buffer`, bind group, render pipeline và
compute pipeline. Khi đăng ký, host cung cấp descriptor tương ứng:

- `TextureResourceDescriptor` cho extent, format, usage, mip và sample count;
- `BufferResourceDescriptor` cho size và usage;
- `PipelineLayoutResourceDescriptor` cho layout signatures;
- `BindGroupResourceDescriptor` cho dynamic offset contract;
- `MeshResourceDescriptor` cho vertex/index contract.

Dùng `insert_owned_texture` khi cần copy, resolve, readback hoặc deferred
destruction. Dùng `insert_texture_with_descriptor` khi registry chỉ cần view.

## Readback và media boundary

Readback trả `RawTextureReadback { bytes, width, height, format }` với row bytes
không padding. Có các đường chính:

- `begin_texture_readback_checked` cho async ticket;
- `read_texture_to_raw_with_format_checked` cho texture/format explicit;
- `read_texture_to_raw_from_registry_checked` cho owned texture lấy format từ
  descriptor trong registry.

Core không lưu PNG/JPEG, không decode asset và không encode video. Higher layer
chuẩn hóa input, chọn color/alpha policy, lấy raw frame và dùng canonical
encoder nếu cần file source-of-truth.

## Tài liệu đọc tiếp

1. [Public usage guide](docs/60-guides/README.md)
2. [Bắt đầu nhanh](docs/60-guides/87-guide-getting-started.md)
3. [Đăng ký resource](docs/60-guides/91-guide-resource-registration.md)
4. [Pipeline và shader](docs/60-guides/88-guide-pipeline-and-shader.md)
5. [Xây dựng graph](docs/60-guides/89-guide-building-a-graph.md)
6. [Execute và lifecycle](docs/60-guides/92-guide-execution-and-lifecycle.md)
7. [API map và versioning](docs/60-guides/93-guide-api-map-and-versioning.md)
8. [Canonical render/media contract](docs/00-foundation/18-canonical-render-and-media-output-contract.md)
9. [Baseline và handoff](docs/70-status/90-validation-boundary-and-clean-baseline.md)

Các file trong `examples/` là executable examples/test harness của workspace;
chúng minh họa integration nhưng không mở rộng boundary của core.
