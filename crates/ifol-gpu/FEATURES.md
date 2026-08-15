# IFOL GPU - Danh Mục Tính Năng & Hướng Dẫn Sử Dụng (V1.0 Production-Ready)

Thư viện `ifol-gpu` là một Lõi Đồ Họa & GPU Task Graph Engine thuần túy, bất khả xâm phạm về bộ nhớ (Panic-free), hỗ trợ 2D, 2.5D, 3D, Motion Graphics và Compute Shader siêu song song.

---

## 1. Bảng Trạng Thái Tính Năng Core (104/104 Tests Passed)

| Thành phần | Khả năng | Trạng thái | Ghi chú |
|---|---|---|---|
| **Backend & Init** | Multi-backend selection (Vulkan, Metal, DX12, WebGPU) | **Đã hoàn thành** | Chạy qua `GpuEngineBuilder` |
| **Backend & Init** | Headless & Window Surface Rendering | **Đã hoàn thành** | Hỗ trợ Offscreen ngầm & Surface presentation |
| **Memory** | Generational Resource Handles | **Đã hoàn thành** | Chống Use-after-free bằng `(index + generation)` |
| **Memory** | Deferred Destruction Queue | **Đã hoàn thành** | Giải phóng VRAM an toàn theo `SubmissionIndex` |
| **Memory** | RingBuffer & FrameContext | **Đã hoàn thành** | Uniform allocation tự động căn lề 256-byte |
| **Memory** | LRU Transient Allocation Pool | **Đã hoàn thành** | Tái sử dụng Texture/Buffer tạm giữa các Pass |
| **Graph Engine** | Multi-level Subgraph Nesting | **Đã hoàn thành** | Lồng ghép đồ thị n-cấp cho Pre-composition |
| **Graph Engine** | Topological Graph Compiler | **Đã hoàn thành** | Tự động duỗi phẳng đồ thị đệ quy |
| **Graph Engine** | Automatic Hazard Edge Modeling | **Đã hoàn thành** | Tự động dò xung đột Read/Write theo Mip/Layer/Aspect |
| **Graph Engine** | Automatic Pass Segmentation | **Đã hoàn thành** | Gom nhóm lệnh cùng Target vào duy nhất 1 RenderPass |
| **Testing Suite** | Automated Cross-Platform Visual Testing (TC01-TC10) | **Đã hoàn thành** | Test Harness đo lường Cold/Warm execution, Memory GC & Caching |
| **Compositing** | Deep 5-Level Recursion SubGraph Compositor | **Đã hoàn thành** | Duỗi phẳng 5 cấp đồ thị đệ quy không tràn stack |
| **Stress & Caching** | TC08-TC30 - Comprehensive Rendering Validation | **Đã hoàn thành** | - [x] **TC08 - Massive Multi-Instance Particle Grid:** 10,000 hạt sao instanced rendering qua `particles_10k.wgsl`, kiểm thử năng lực tải cực hạn của draw call GPU.<br>- [x] **TC08.5 - 100% Procedural & Modular Anime Night Sky Prefab Scene:** Khung cảnh bầu trời đêm anime hoàn chỉnh dựng 100% từ Procedural Shaders và Props đơn lẻ.<br>- [x] **TC09 - Pipeline Caching & RenderBundle Execution:** Chạy lặp 10 frame với tập lệnh đã cache, xác thực tốc độ ổn định đạt **~980µs** cho 10,000 instance.<br>- [x] **TC10 - Zero-Panic Fallback Pipeline:** Kiểm thử khả năng phục hồi lỗi khi thiếu tài nguyên.<br>- [x] **TC11 - Multi-Viewport Split-Screen & Camera Isolation:** Render song song 2 Viewport/Camera độc lập bảo toàn tỉ lệ.<br>- [x] **TC12 - Fine Chroma Key Edge Despill & Smooth Alpha Feathering:** Bóc tách phông xanh lá chính xác trên 5 vật thể phức tạp.<br>- [x] **TC13 - 2-Pass Separable Gaussian Blur Filter & Depth of Field:** Kỹ thuật xóa phông DOF điện ảnh với bộ lọc Gaussian 9-tap tách rời.<br>- [x] **TC14 - Cinematic Color Grading & ACES Filmic Tone Mapping:** Pipeline phân loại màu sắc điện ảnh với đường cong ACES Filmic.<br>- [x] **TC15 - Animated Particle Physics Simulator:** Mô phỏng 200 hạt tuyết chuyển động vật lý thời gian thực.<br>- [x] **TC27 - GodRays (Volumetric Light Shafts):** Hiệu ứng Tia Sáng Radial Blur, đo năng lực tính toán vòng lặp lấy mẫu (heavy texture sampling loop) trong Fragment Shader.<br>- [x] **TC28 - Ripple (Water/Shockwave Distortion):** Hiệu ứng lượn sóng nước/xung kích lan tỏa, bóp méo UV theo hàm Sin/Cos.<br>- [x] **TC29 - CRT & VHS Monitor Filter:** Hiệu ứng màn hình cong CRT cũ kỹ, kết hợp Scanlines, Vignette và Chromatic Aberration.<br>- [x] **TC30 - Dissolve / Burn Transition:** Hiệu ứng tan biến/cháy giấy sử dụng lệnh `discard` và Noise Map làm bản đồ độ cao. |
| **Compositing** | GPU Chroma Key & UV Crop Pipeline | **Đã hoàn thành** | Shader bóc tách phông xanh lá và trích xuất Sprite UV |
| **Compositing** | Depth Testing & Alpha Blending Interaction | **Đã hoàn thành** | Kiểm thử Z-Buffer culling và translucent blending |
| **Compositing** | Multi-Pass Chained SubGraph Compositor | **Đã hoàn thành** | Chuỗi 3 pass liên hoàn kết hợp background và props |
| **Commands** | Draw Commands (Indexed, Procedural, Indirect) | **Đã hoàn thành** | Hỗ trợ vẽ lưới tam giác, procedural và indirect draw |
| **Commands** | Compute Commands (Direct & Indirect) | **Đã hoàn thành** | Hỗ trợ GPU Physics, Particles, Image Processing |
| **Commands** | Copy Commands (Buffer & Texture) | **Đã hoàn thành** | Hỗ trợ async readback xuất video MP4 |
| **Validation** | MSAA Resolve & Depth/Stencil Aspect Validation | **Đã hoàn thành** | Kiểm tra tương thích phần cứng, báo lỗi Typed Error |
| **Extensions** | Plugin & Custom Extension Dispatcher | **Đã hoàn thành** | Đăng ký lệnh tùy biến qua `DispatchRegistry` |

---

## 2. Hướng Dẫn Sử Dụng (Usage Examples)

### Ví Dụ 1: Khởi Tạo Engine & Đăng Ký Tài Nguyên

```rust
use ifol_gpu::api::builder::GpuEngineBuilder;
use ifol_gpu::resources::registry::ResourceRegistry;
use ifol_gpu::resources::descriptors::TextureResourceDescriptor;

#[pollster::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Khởi tạo Engine
    let engine = GpuEngineBuilder::new()
        .with_backends(wgpu::Backends::PRIMARY)
        .build()
        .await?;

    let mut registry = ResourceRegistry::new();

    // 2. Nạp Texture vào Registry (Nhận Generational Handle)
    let texture_desc = TextureResourceDescriptor {
        width: 1920,
        height: 1080,
        depth_or_array_layers: 1,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        mip_level_count: 1,
        sample_count: 1,
    };
    
    // Đăng ký nhận Handle an toàn
    let texture_handle = registry.create_texture(&engine, &texture_desc)?;

    Ok(())
}
```

### Ví Dụ 2: Xây Dựng Render Graph & Chạy Lệnh

```rust
use ifol_gpu::render::graph::{RenderGraph, RenderTarget};
use ifol_gpu::render::node::{RenderNodePool, RenderNode, DrawCommand, DrawAction};
use ifol_gpu::render::compiler::GraphCompiler;

fn render_frame(
    engine: &GpuEngine,
    registry: &ResourceRegistry,
    pipeline_handle: PipelineHandle,
    mesh_handle: MeshHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);

    // 1. Tạo Node vẽ
    let draw_node_id = pool.add_render_node(RenderNode {
        name: "Draw Main Scene".into(),
        commands: vec![DrawCommand {
            pipeline: pipeline_handle,
            bind_groups: vec![],
            vertex_buffers: vec![(0, mesh_handle.vertex_buffer, 0)],
            index_buffer: Some((mesh_handle.index_buffer, 0, wgpu::IndexFormat::Uint16)),
            viewport: None,
            scissor: None,
            action: DrawAction::Indexed {
                index_count: 6,
                instance_count: 1,
                first_index: 0,
                base_vertex: 0,
                first_instance: 0,
            },
        }],
        declared_accesses: vec![],
    });

    graph.add_node(draw_node_id);

    // 2. Biên dịch và Thực thi (Chống panic, tự chèn Barriers)
    let compiler = GraphCompiler::new();
    let submission_index = compiler.execute_checked(engine, registry, &mut pool, &graph)?;

    println!("Frame submitted successfully with Index: {:?}", submission_index);
    Ok(())
}
```

### Ví Dụ 3: Hủy Tài Nguyên An Toàn (Deferred Destruction)

```rust
use ifol_gpu::memory::deferred::DeferredDestructionQueue;

fn cleanup_texture(
    registry: &mut ResourceRegistry,
    queue: &mut DeferredDestructionQueue,
    texture_handle: TextureHandle,
    last_used_submission: wgpu::SubmissionIndex,
) {
    // Đưa Texture vào hàng đợi chờ GPU hoàn thành vẽ
    registry.defer_owned_texture_destruction(&texture_handle, last_used_submission, queue);
}
```
## Hỗ Trợ Đồ Họa Nâng Cao (VFX / Motion Graphics)
- **Light Sweep (TC31):** Quét sáng góc chéo sử dụng Additive Blending và toán học.
- **Page Curl 3D (TC32):** Biến dạng Cylinder Projection tạo hiệu ứng lật trang 3D với đổ bóng.
- **Pixelation/Mosaic (TC33):** Screen coordinate snapping bằng thuật toán Floor.
- **Directional Blur (TC34):** Vòng lặp lấy mẫu dọc theo vector đa hướng.
- **Halftone Comic Filter (TC35):** Chuyển đổi Luminance thành chấm đen sử dụng SDF.

## Bộ Lọc Đặc Biệt & Biến Dạng (Distortion & Post-processing)
- **Radial Blur (TC36):** Làm nhòe theo hướng tỏa ra từ tâm màn hình.
- **Chromatic Aberration (TC37):** Phân tách 3 kênh RGB mô phỏng quang sai ống kính.
- **Kaleidoscope (TC38):** Phản chiếu hình ảnh theo hệ tọa độ cực tạo kính vạn hoa.
- **Scanlines Hologram (TC39):** Giả lập sọc ngang màn hình CRT/Hologram bằng sóng sine.
- **Vignette & Film Grain (TC40):** Tối góc và nhiễu phim cổ điển.

## Tính năng Compositor Nâng cao & Thích ứng Đồ họa (Phase 7: TC41-TC45)
- **Auto Aspect Ratio & Background Blur Fill (TC41):** Tự động chuyển đổi tỷ lệ khung hình ngang (16:9) sang dọc (9:16) bằng thuật toán phóng đại nền, làm mờ Gaussian và đổ bóng lên khung trung tâm.
- **Full-Frame HDR Bloom & Emissive Glow (TC42):** Trích xuất vùng phát sáng (Emissive) và làm mờ lan tỏa ra toàn bộ màn hình (800x600), cộng dồn màu quang học (Additive Blending) không bị cản viền vuông.
- **Dual-Layer Track Matte (TC43):** Hỗ trợ 4 chế độ mặt nạ Stencil giữa 2 texture động độc lập: Alpha Matte, Inverted Alpha, Luma Matte, Inverted Luma.
- **Anamorphic Lens Flare & Streak (TC44):** Lấy mẫu quang sai trục ngang 1D dải rộng (33 taps) có khử viền đen biên UV (Boundary Falloff Clamping).
- **Frosted Glassmorphism Panel (TC45):** Lấy mẫu ngược khung cảnh nền (Backdrop), làm mờ kính mờ kết hợp khúc xạ viền SDF và phản xạ viền Specular Fresnel.

### Hướng dẫn sử dụng Track Matte & Glassmorphism (Usage Examples)

```rust
// 1. Áp dụng Track Matte (Layer A dùng Layer B làm mặt nạ trong suốt)
let dual_bg = harness.create_dual_texture_bind_group(tex_base, tex_matte_mask, "TrackMatte");
let matte_uniform = TrackMatteUniform {
    matte_type: 0.0, // 0 = Alpha Matte, 2 = Luma Matte
    opacity: 1.0,
    _pad0: 0.0,
    _pad1: 0.0,
};

// 2. Áp dụng Glassmorphism UI Panel trên nền Backdrop
let glass_uniform = GlassUniform {
    panel_center: [0.5, 0.5],
    panel_size: [0.25, 0.25],
    corner_radius: 0.03,
    blur_amount: 3.0,
    refraction_strength: 0.02,
    border_thickness: 0.005,
};
```

## Tính năng Phân tích & Đồ họa Điện ảnh Chuyên nghiệp (Phase 8: TC46-TC50)
- **Selective Color Isolation (TC46):** Phân tích không gian màu HSV và góc Hue hình tròn để tách màu chọn lọc (Sin City Effect), chuyển toàn cảnh sang Grayscale và giữ lại màu giáp đỏ/hồng.
- **Motion Echo & Afterimage Ghosting (TC47):** Giả lập tàn ảnh tốc độ cao với 5 lớp bóng ma phân rã mờ dần (Exponential Decay) và xoay chuyển sắc thái màu quang học (Spectral Hue Trail).
- **Cinematic Bokeh Depth of Field (TC48):** Tính toán mặt phẳng hội tụ tiêu cự (Circle of Confusion CoC) kết hợp lấy mẫu đĩa xoắn Fermat Golden Angle làm các đốm sáng bung nở thành đĩa tròn Bokeh quang học.
- **Animated Trim Paths & Dashed Vector Stroke (TC49):** Mô phỏng tính năng Trim Paths của After Effects với đường viền nét đứt neon tự động chạy và cắt theo tỷ lệ phần trăm chu vi SDF.
- **Exposure Inspector (Zebra Stripes & False Color) (TC50):** Bộ công cụ phân tích quang học chuyên dụng: Quét mức IRE phơi sáng, tô màu nhiệt giả lập (ARRI False Color Heatmap) và vẽ sọc ngựa vằn chuyển động tại vùng cháy sáng (> 80% IRE).

### Hướng dẫn sử dụng Bokeh DoF & Exposure Inspector (Usage Examples)

```rust
// 1. Áp dụng Cinematic Bokeh Depth of Field
let bokeh_uniform = BokehUniform {
    focus_point: [0.5, 0.5], // Vùng tâm sắc nét
    focus_radius: 0.2,       // Bán kính vùng nét
    max_blur: 3.5,           // Kích thước đĩa Bokeh
    highlight_boost: 6.0,    // Độ rực của đĩa sáng Bokeh
    _pad0: 0.0,
};

// 2. Bật công cụ đo phơi sáng False Color & Zebra
let exposure_uniform = ExposureUniform {
    zebra_threshold: 0.80,   // Sọc ngựa vằn tại vùng sáng > 80%
    zebra_speed: 2.0,        // Tốc độ chuyển động vạch sọc
    time: 1.0,
    mode: 0.0,               // 0 = Chia đôi màn hình (Trái: Zebra, Phải: False Color)
};
```

## Tính năng Render Pipeline Chuyên sâu & Edge Cases (Phase 9: TC51-TC55)
- **Texture Atlas Sub-pixel Bleed Prevention (TC51):** Kỹ thuật kẹp nửa Texel (`Half-Texel UV Inset Clamping`) loại bỏ hoàn toàn hiện tượng lem viền giữa các Sprite kề nhau trên cùng một Atlas Sheet khi nội suy tuyến tính (Linear Filtering).
- **Soft Particle Depth Fading (TC52):** Mô phỏng quả cầu năng lượng plasma tiếp xúc và giao thoa mềm mại với hình học nhân vật, khử vết cắt cứng (Hard Intersection) qua Z-buffer Depth Attachment.
- **Advanced 8 Blend Modes Matrix (TC53):** Ma trận 8 chế độ hòa trộn lớp chuẩn After Effects / Photoshop: Normal, Multiply, Screen, Overlay, Hard Light, Soft Light, Color Dodge, Difference.
- **High-Density Procedural 3D Flag Mesh (TC54):** Nạp và thực thi `VertexBuffer` + `IndexBuffer` thực tế (32x32 lưới = 1,089 đỉnh, 6,144 chỉ số) kết hợp biến dạng sóng 3D trong Vertex Shader và chiếu sáng bề mặt Phong Lighting.
- **Dual Kawase Fast Bloom Filter (TC55):** Thuật toán làm mờ phân cấp 2 tầng (Downsample 400x300 và Upsample 800x600) lấy mẫu 8 điểm đa hướng, đạt hiệu ứng phát sáng diện rộng cực nhanh ở 60+ FPS.

### Hướng dẫn sử dụng Mesh 3D & Blend Modes (Usage Examples)

```rust
// 1. Đăng ký Mesh đa giác với Vertex & Index Buffer
harness.registry.insert_mesh_with_descriptor(
    mesh_id,
    (vertex_buffer, Some((index_buffer, wgpu::IndexFormat::Uint16)), index_count),
    MeshResourceDescriptor {
        vertex_buffer_size: vb_size,
        vertex_count: num_vertices,
        index_buffer_size: Some(ib_size),
        index_format: Some(wgpu::IndexFormat::Uint16),
    },
)?;

// 2. Gọi lệnh vẽ Indexed Draw
let draw_cmd = DrawCommand::new(pipeline_id, DrawAction::Indexed {
    mesh: mesh_id,
    index_range: 0..index_count,
    instance_range: 0..1,
});
```
