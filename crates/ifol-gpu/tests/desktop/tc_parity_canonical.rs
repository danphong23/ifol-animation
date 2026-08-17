use ifol_gpu::backend::GpuEngineBuilder;
use ifol_gpu::execution::RenderGraphExecutor;
use ifol_gpu::graph::{RenderGraph, RenderNodePool, RenderTarget};
use ifol_gpu::resources::{ResourceRegistry, TextureHandle, TextureResourceDescriptor};
use std::fs;
use std::time::Instant;

#[test]
fn run_canonical_offscreen_parity_probe() {
    let _ = env_logger::builder().is_test(true).try_init();

    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    let width = 800;
    let height = 600;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("canonical-parity-probe"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let handle = TextureHandle(1);
    let mut registry = ResourceRegistry::new();
    registry
        .insert_owned_texture(
            handle,
            texture,
            TextureResourceDescriptor {
                width,
                height,
                depth_or_array_layers: 1,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                mip_level_count: 1,
                sample_count: 1,
            },
            4096,
        )
        .unwrap();

    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: handle,
        width,
        height,
    })
    .with_clear_color([0.03, 0.04, 0.07, 1.0]);
    let mut pool = RenderNodePool::new();
    let executor = RenderGraphExecutor::new();
    let started = Instant::now();
    let submission = executor
        .execute_checked(&engine, &registry, &mut pool, &graph)
        .expect("Canonical graph execution failed");
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
    let elapsed = started.elapsed();

    let readback = engine
        .read_texture_to_raw_from_registry_checked(&registry, &handle)
        .expect("Canonical readback failed");
    assert_eq!(readback.format, format);
    assert_eq!(readback.bytes.len(), (width * height * 4) as usize);

    fs::create_dir_all("tests/outputs/desktop").unwrap();
    fs::write(
        "tests/outputs/desktop/canonical_parity_rgba8unorm.bin",
        &readback.bytes,
    )
    .unwrap();
    fs::write(
        "tests/outputs/desktop/canonical_parity_rgba8unorm.json",
        serde_json::to_string_pretty(&serde_json::json!({
            "width": width,
            "height": height,
            "format": "Rgba8Unorm",
            "clear": [0.03, 0.04, 0.07, 1.0],
            "render_time_ms": elapsed.as_secs_f64() * 1000.0,
        }))
        .unwrap(),
    )
    .unwrap();
}
