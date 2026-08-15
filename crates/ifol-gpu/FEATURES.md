# IFOL GPU - Feature Specification & API Contract (V1.0 Production)

`ifol-gpu` là Lõi Đồ Họa & GPU Task Graph Engine hiệu năng cao, bất khả xâm phạm về bộ nhớ (Panic-free), thiết kế theo kiến trúc phi trạng thái (Stateless/Agnostic) hỗ trợ 2D, 2.5D, 3D Motion Graphics và Compute Shader.

---

## 1. Bất Biến Kiến Trúc & Hợp Đồng Kỹ Thuật (Core Invariants & Contracts)

1. **An toàn bộ nhớ tuyệt đối (Generational Handles):**
   - Mọi tài nguyên GPU (`Texture`, `Buffer`, `BindGroup`, `Pipeline`) đều được tham chiếu gián tiếp qua Generational Handle `(Index + Generation)` trong `ResourceRegistry`.
   - Ngăn chặn hoàn toàn lỗi Use-after-free, Double-free và Pointer Dangling.

2. **Cơ chế RingBuffer Uniform 256-Byte Alignment:**
   - Dữ liệu Animation/Transform được nạp liên tục vào Ring Buffer với Dynamic Offset.
   - Khi dịch chuyển, xoay, scale vật thể: **Không cần tạo mới BindGroup, CPU overhead $\approx 0\text{ms}$**.

3. **Thuật toán RenderGraph Đệ Quy (Multi-Level SubGraph):**
   - Hỗ trợ Pre-composition lồng ghép $N$ cấp tựa After Effects.
   - Tự động phân tách Pass (Pass Segmentation) và sắp xếp thứ tự thực thi topo (Topological Sorting) để xuất hình ảnh phụ trước khi làm đầu vào cho Pass chính.

4. **Tái sử dụng tài nguyên (Transient Allocation & Deferred Destruction):**
   - Texture trung gian tự động được thu hồi và tái sử dụng qua `LRU Transient Pool`.
   - Hủy tài nguyên an toàn theo `SubmissionIndex` của GPU queue, không bao giờ hủy buffer khi GPU đang đọc.

---

## 2. Bề Mặt API Công Khai (Public API Surface)

| Module | Structs / Enums chính | Vai trò |
| :--- | :--- | :--- |
| **`api`** | `GpuEngineBuilder`, `GpuEngine`, `SurfaceContext` | Khởi tạo WGPU backend (Vulkan, Metal, DX12, WebGPU), tạo Surface/Offscreen context. |
| **`resources`** | `ResourceRegistry`, `TextureHandle`, `BufferHandle`, `BindGroupHandle`, `PipelineHandle` | Quản lý vòng đời tài nguyên, Descriptor và Generational SlotMap. |
| **`graph`** | `RenderGraph`, `SubGraph`, `DrawCommand`, `DrawAction`, `RenderTarget` | Xây dựng cây tác vụ render, khai báo Clear/LoadOp, BindGroup và Viewport. |
| **`execution`** | `RenderGraphExecutor`, `PooledCommandEncoder` | Biên dịch và thực thi RenderGraph, tối ưu RenderBundle và Record lệnh. |

---

## 3. Hướng Dẫn Sử Dụng Thực Chiến (Usage Cookbooks)

### Recipe 1: Khởi Tạo Engine & Đăng Ký Tài Nguyên
```rust
use ifol_gpu::api::builder::GpuEngineBuilder;
use ifol_gpu::resources::registry::ResourceRegistry;
use ifol_gpu::resources::descriptors::TextureResourceDescriptor;

#[pollster::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Khởi tạo GpuEngine
    let engine = GpuEngineBuilder::new()
        .with_backends(wgpu::Backends::PRIMARY)
        .build()
        .await?;

    let mut registry = ResourceRegistry::new();

    // 2. Tạo Texture Target
    let (texture_handle, _view_handle) = registry.create_texture(
        engine.device(),
        &TextureResourceDescriptor {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: vec![],
        },
        Some("Main Output Target"),
    )?;

    Ok(())
}
```

### Recipe 2: Dựng RenderGraph & Thực Thi Vẽ
```rust
use ifol_gpu::graph::{RenderGraph, RenderTarget, DrawCommand, DrawAction, RenderNodePool};
use ifol_gpu::execution::RenderGraphExecutor;

fn render_frame(
    engine: &ifol_gpu::api::GpuEngine,
    registry: &ifol_gpu::resources::ResourceRegistry,
    executor: &mut RenderGraphExecutor,
    pool: &mut RenderNodePool,
    target_handle: ifol_gpu::resources::TextureHandle,
    pipeline_handle: ifol_gpu::resources::PipelineHandle,
    texture_bg: ifol_gpu::resources::BindGroupHandle,
    uniform_bg: ifol_gpu::resources::BindGroupHandle,
) {
    // 1. Khởi tạo RenderGraph hướng tới Target
    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: target_handle,
        width: 1920,
        height: 1080,
    }).with_clear_color([0.05, 0.05, 0.08, 1.0]);

    // 2. Thêm Batch vẽ
    graph.add_batch(pool, vec![
        DrawCommand::new(pipeline_handle, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
            .with_bind_group(0, texture_bg, Vec::new())
            .with_bind_group(1, uniform_bg, Vec::new())
    ]);

    // 3. Thực thi Graph
    let submission_index = executor.execute(engine, registry, pool, &graph).expect("Execution failed");
    engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission_index),
        timeout: None,
    });
}
```

### Recipe 3: Kỹ Thuật VFX Nâng Cao (MRT, Stencil, Ping-Pong)
- **Multiple Render Targets (MRT):** Đăng ký Fragment Shader xuất đồng thời `@location(0)` Albedo và `@location(1)` Emissive/Normal Mask trong 1 pass.
- **Hardware Stencil Portal:** Cấu hình `wgpu::StencilState` với `IncrementClamp` (ghi mặt nạ) và `NotEqual` (vẽ nội dung bị giới hạn) để cắt tỉa hình học không cần alpha blend.
- **Multi-Pass Ping-Pong Loop:** Khởi tạo RenderGraph với `clear_color: None` để kích hoạt `wgpu::LoadOp::Load`, luân chuyển Texture giữa 2 Target tạo vệt đuôi Motion Trail / Blur.

### Recipe 4: Tính Toán Song Song Trên GPU (GPGPU Compute Pipeline)
```rust
use ifol_gpu::graph::{ComputeCommand, RenderGraph, RenderNodePool, RenderTarget};
use ifol_gpu::resources::handle::ComputePipelineHandle;

fn run_compute_task(
    engine: &ifol_gpu::api::GpuEngine,
    registry: &ifol_gpu::resources::ResourceRegistry,
    executor: &mut ifol_gpu::execution::RenderGraphExecutor,
    pool: &mut RenderNodePool,
    compute_pipeline: ComputePipelineHandle,
    storage_bind_group: ifol_gpu::resources::BindGroupHandle,
    element_count: u32,
) {
    let mut graph = RenderGraph::new(RenderTarget::Screen);

    // 1. Điều phối Workgroups (Ví dụ: 64 threads / workgroup)
    let workgroups_x = (element_count + 63) / 64;
    graph.add_compute_batch(pool, vec![
        ComputeCommand::new(compute_pipeline, [workgroups_x, 1, 1])
            .with_bind_group(0, storage_bind_group, Vec::new()),
    ]);

    // 2. Thực thi Compute Batch không qua Rasterizer
    let submission_index = executor.execute(engine, registry, pool, &graph).expect("Compute execution failed");
    engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission_index),
        timeout: None,
    });
}
```

---

## 4. Ma Trận Kiểm Chứng Hệ Thống (Verification Matrix - 65 Desktop TCs)

Tất cả 65 bài kiểm thử Desktop (60 Render TCs + 5 Compute TCs) đều được tự động đo lường hiệu năng (Cold/Warm run), kiểm tra lỗi GPU, xác thực số học và trích xuất hình ảnh kiểm chứng trực quan:

| Nhóm Kiểm Thử | Dải Test Cases | Khả năng kiểm chứng | Báo cáo chi tiết |
| :--- | :--- | :--- | :--- |
| **Core & Lifecycle** | TC01 $\rightarrow$ TC10 | Clear screen, Single Quad, Z-Buffer, Alpha Blend, Interleaved Pass, GC VRAM, Texture Cache, Fallback pipeline. | [`tests/reports/tc01_empty_scene_report.md`](tests/reports/tc01_empty_scene_report.md) $\rightarrow$ `tc10` |
| **Compositing & Post-FX** | TC11 $\rightarrow$ TC20 | Multi-viewport split, Chroma key despill, Gaussian blur, Filmic ACES LUT, Particle physics, SDF shapes, Outline, Audio spectrum, 2.5D perspective. | [`tests/reports/tc11_viewport_split_report.md`](tests/reports/tc11_viewport_split_report.md) $\rightarrow$ `tc20` |
| **Motion Graphics VFX** | TC21 $\rightarrow$ TC30 | Track matte masking, 10k Instancing, Color replace, Grid mesh distortion, Rim light, Glitch, God rays, Ripple, CRT VHS, Dissolve. | [`tests/reports/tc21_track_matte_report.md`](tests/reports/tc21_track_matte_report.md) $\rightarrow$ `tc30` |
| **Stylization & Filters** | TC31 $\rightarrow$ TC40 | Light sweep, Page curl, Pixelation, Directional blur, Halftone, Radial blur, Chromatic aberration, Kaleidoscope, Scanlines, Vignette. | [`tests/reports/tc31_light_sweep_report.md`](tests/reports/tc31_light_sweep_report.md) $\rightarrow$ `tc40` |
| **VFX Cao Cấp & Tone Mapping** | TC41 $\rightarrow$ TC50 | Aspect ratio fill, HDR bloom, Luma/Alpha matte, Anamorphic flare, Glassmorphism, Selective color, Motion echo, Bokeh DOF, Trim paths, Exposure heatmap. | [`tests/reports/tc41_aspect_fill_report.md`](tests/reports/tc41_aspect_fill_report.md) $\rightarrow$ `tc50` |
| **Phần Cứng & Edge Cases** | TC51 $\rightarrow$ TC60 | Atlas half-texel clamp, Soft particles, 8 Blend modes, 3D flag mesh, Dual Kawase blur, Dynamic target resize, Stencil mask, MRT G-Buffer, Sampler wrapping, Ping-Pong loop. | [`tests/reports/tc51_atlas_clamp_report.md`](tests/reports/tc51_atlas_clamp_report.md) $\rightarrow$ `tc60` |
| **GPGPU & Compute Pipeline** | **TC61 $\rightarrow$ TC65** | **Storage Buffer Arithmetic (10k threads), 2D Storage Texture Read/Write (Sobel/Invert), 100k Galaxy Particle Simulation, Audio FFT 64-Bin Spectrum, Workgroup Shared Memory (`var<workgroup>`) Fast Blur.** | [`tests/reports/tc61_compute_buffer_math_report.md`](tests/reports/tc61_compute_buffer_math_report.md) $\rightarrow$ [`tc65`](tests/reports/tc65_workgroup_blur_report.md) |

> 💡 **Chi tiết ảnh render và số liệu benchmark:** Xem toàn bộ 65 báo cáo độc lập tại thư mục [`crates/ifol-gpu/tests/reports/`](tests/reports/).

