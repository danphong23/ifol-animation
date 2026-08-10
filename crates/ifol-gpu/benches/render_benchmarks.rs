use criterion::{criterion_group, criterion_main, Criterion};
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{RenderGraph, RenderNode, RenderTarget, ResourceRegistry, TextureHandle, RenderGraphExecutor};

fn bench_clear_screen(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    
    // Tạo 1 Texture ảo trên VRAM để làm bia tập vẽ (Target)
    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // Đăng ký tài nguyên vào Registry
    let mut registry = ResourceRegistry::new();
    let tex_handle = TextureHandle(1);
    registry.textures.insert(tex_handle, target_view);

    // Dựng luồng RenderGraph (Chỉ Xóa Màn Hình - Không lệnh vẽ)
    let mut graph = RenderGraph::new();
    let target = RenderTarget {
        color_attachments: vec![tex_handle],
        depth_attachment: None,
    };
    graph.add_node(RenderNode::new("ClearPass", target));

    // Bắt đầu Benchmark!
    c.bench_function("bench_clear_screen", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); // Đồng bộ ép GPU chạy xong mới đếm thời gian
        })
    });
}

criterion_group!(benches, bench_clear_screen);
criterion_main!(benches);
