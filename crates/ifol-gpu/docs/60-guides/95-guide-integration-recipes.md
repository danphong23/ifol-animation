# Mẫu tích hợp

Các recipe này chỉ minh họa ranh giới host–core. Host tạo dữ liệu, texture,
pipeline và policy; `ifol-gpu` chỉ nhận descriptor, handle và graph contract.

## 1. Khởi tạo engine headless

```rust,ignore
use ifol_gpu::backend::{GpuEngine, GpuEngineBuilder, GpuError};

pub async fn create_engine() -> Result<GpuEngine<'static>, GpuError> {
    GpuEngineBuilder::new().build().await
}
```

Host có thể chọn backend, power preference, required features/limits hoặc truyền
surface vào builder. Window/event loop không thuộc crate.

## 2. Nạp texture từ higher layer

Higher layer decode và chuẩn hóa asset thành bytes canonical trước. Sau đó host:

1. tạo `wgpu::Texture` với format, extent và usage đã chọn;
2. ghi bytes vào texture bằng `wgpu::Queue`;
3. tạo view/bind group/pipeline tương ứng;
4. đăng ký texture bằng `insert_texture_with_descriptor` hoặc
   `insert_owned_texture`;
5. đưa handle vào graph.

Với video YUV/NV12, media layer truyền các plane texture và shader conversion
do host chọn. Core không biết codec, BT.709, range hay color policy.

## 3. Dựng và execute graph

```rust,ignore
use ifol_gpu::execution::RenderGraphExecutor;
use ifol_gpu::graph::{RenderGraph, RenderNodePool, RenderTarget};

pub fn execute_frame(
    engine: &ifol_gpu::backend::GpuEngine<'_>,
    registry: &ifol_gpu::resources::ResourceRegistry,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
) -> Result<wgpu::SubmissionIndex, ifol_gpu::execution::RenderGraphValidationError> {
    RenderGraphExecutor::new().execute_checked(engine, registry, pool, graph)
}
```

Production host nên dùng `execute_checked` hoặc biến thể có report và xử lý
`Result`. Sau khi nhận submission, host giữ resource cho tới khi GPU hoàn tất
theo policy lifetime của mình.

## 4. Extension

Extension phải khai báo resource usage và được đăng ký qua
`ExtensionDispatchRegistry`. Extension không được lách validation để đưa domain
semantics, decoder hoặc encoder vào core. Xem
[guide extension](90-guide-public-api-and-extensions.md).

## 5. Readback và xuất file

Dùng `read_texture_to_raw_from_registry_checked` cho owned texture khi cần raw
frame. Higher layer sau đó mới áp dụng color/alpha policy và encoder canonical
để tạo PNG, JPEG, EXR hoặc video. Preview surface/canvas không được coi là
source of truth.

Các lỗi cần được phân loại ở host thành decode/input, graph/validation,
execution/readback hoặc encode/output; không biến lỗi validation thành một
fallback node ngầm trong core.
