# Hướng Dẫn Tích Hợp & Mẫu Code Thực Tế (Integration Guide & Recipes)

Tài liệu này cung cấp các đoạn mã mẫu thực tế chuẩn công nghiệp để các Crate như `ifol-app-core`, `ifol-ecs`, `ifol-media` hoặc các Plugin mở rộng dễ dàng tích hợp và sử dụng `ifol-gpu`.

---

## 🍳 Recipe 1: Khởi Tạo Engine Đa Nền Tảng (Desktop & Web)

```rust
use ifol_gpu::backend::{GpuEngine, GpuEngineBuilder};

pub async fn init_gpu_engine() -> Result<GpuEngine<'static>, Box<dyn std::error::Error>> {
    let engine = GpuEngineBuilder::new()
        .with_power_preference(wgpu::PowerPreference::HighPerformance)
        .with_required_limits(wgpu::Limits::default())
        .build()
        .await?;
        
    println!("iFol GPU Engine initialized on: {:?}", engine.adapter().get_info().name);
    Ok(engine)
}
```

---

## 🍳 Recipe 2: Đưa Frame Video NV12 từ `ifol-media` vào `ifol-gpu`

Được thiết kế để nhận frame từ FFmpeg (Desktop) hoặc WebCodecs (Web) và render chuyển hệ màu BT.709 lên màn hình:

```rust
use ifol_gpu::resources::{TextureHandle, TextureResourceDescriptor, ResourceRegistry};

pub fn upload_video_nv12_frame(
    engine: &GpuEngine,
    registry: &mut ResourceRegistry,
    y_plane_bytes: &[u8],
    uv_plane_bytes: &[u8],
    width: u32,
    height: u32,
) -> (TextureHandle, TextureHandle) {
    let y_h = TextureHandle(1001);
    let uv_h = TextureHandle(1002);

    // 1. Nạp Y-Plane (R8Unorm)
    let y_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("video_y_plane"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo { texture: &y_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        y_plane_bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width), rows_per_image: Some(height) },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    registry.insert_owned_texture(y_h, y_tex, TextureResourceDescriptor {
        width, height, depth_or_array_layers: 1, format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1, sample_count: 1,
    }, (width * height) as u64).unwrap();

    // 2. Nạp UV-Plane (Rg8Unorm)
    let uv_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("video_uv_plane"),
        size: wgpu::Extent3d { width: width / 2, height: height / 2, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rg8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo { texture: &uv_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        uv_plane_bytes,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width), rows_per_image: Some(height / 2) },
        wgpu::Extent3d { width: width / 2, height: height / 2, depth_or_array_layers: 1 },
    );
    registry.insert_owned_texture(uv_h, uv_tex, TextureResourceDescriptor {
        width: width / 2, height: height / 2, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rg8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        mip_level_count: 1, sample_count: 1,
    }, (width * height / 2) as u64).unwrap();

    (y_h, uv_h)
}
```

---

## 🍳 Recipe 3: Viết Native GPU Plugin Extension

Cho phép bên thứ ba (Machine Learning inference, custom GPU particle simulator) can thiệp trực tiếp vào Command Buffer:

```rust
use ifol_gpu::extensions::{ExtensionDescriptor, ExtensionDispatcher, ExtensionExecutionContext, ExtensionExecutionError, ExtensionId};
use ifol_gpu::graph::{ResourceUsage, GraphResource, ResourceAccess, ResourceSubresource};
use std::sync::Arc;

pub struct GaussianBlurVfxExtension {
    descriptor: ExtensionDescriptor,
}

impl ExtensionDispatcher for GaussianBlurVfxExtension {
    fn descriptor(&self) -> ExtensionDescriptor {
        self.descriptor.clone()
    }

    fn encode(&self, mut context: ExtensionExecutionContext<'_, '_>) -> Result<(), ExtensionExecutionError> {
        let encoder = context.encoder();
        // Chèn native pass hoặc barrier tùy chỉnh
        println!("Custom Plugin executing on CommandEncoder!");
        Ok(())
    }
}
```

---

## 🍳 Recipe 4: Mẫu Xây Dựng `RenderGraph` từ ECS World

```rust
use ifol_gpu::graph::{RenderGraph, RenderNodePool, RenderTarget, DrawCommand, DrawAction};
use ifol_gpu::resources::PipelineHandle;

pub fn build_frame_graph(
    pool: &mut RenderNodePool,
    target_handle: ifol_gpu::resources::TextureHandle,
    sprite_pipeline: PipelineHandle,
    sprite_count: u32,
) -> RenderGraph {
    let mut graph = RenderGraph::new(RenderTarget::Offscreen {
        color: target_handle,
        width: 1920,
        height: 1080,
    }).with_clear_color([0.05, 0.05, 0.08, 1.0]);

    // Thêm Node vẽ Sprite
    let draw_cmd = DrawCommand::new(
        sprite_pipeline,
        DrawAction::Procedural {
            vertex_count: 4,
            instance_range: 0..sprite_count,
        },
    );

    let node_draw = pool.alloc_batch(vec![draw_cmd]);
    graph.add_node_id(node_draw);

    graph
}
```
