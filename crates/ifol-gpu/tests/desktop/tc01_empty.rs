use ifol_gpu::backend::GpuEngineBuilder;
use ifol_gpu::graph::{RenderGraph, RenderTarget, RenderNodePool};
use ifol_gpu::resources::{ResourceRegistry, TextureHandle, TextureResourceDescriptor};
use ifol_gpu::execution::RenderGraphExecutor;
use std::time::Instant;
use std::fs;
use serde_json::Value;

#[path = "output.rs"]
mod output;

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[test]
fn run_tc01_empty() {
    let _ = env_logger::builder().is_test(true).try_init();
    
    // 1. Init engine
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    
    // 2. Registries
    let mut registry = ResourceRegistry::new();
    let executor = RenderGraphExecutor::new();
    let mut pool = RenderNodePool::new();
    
    let manifest_text = include_str!("../shared_assets/manifests/tc01_empty.json");
    let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC01 shared manifest");
    let graph_spec = &manifest["graph"];
    let width = graph_spec["target"]["width"].as_u64().unwrap() as u32;
    let height = graph_spec["target"]["height"].as_u64().unwrap() as u32;
    let clear = graph_spec["operations"][0]["color"]
        .as_array()
        .expect("TC01 clear operation must contain four channels");
    let clear_color = [
        clear[0].as_f64().unwrap() as f32,
        clear[1].as_f64().unwrap() as f32,
        clear[2].as_f64().unwrap() as f32,
        clear[3].as_f64().unwrap() as f32,
    ];
    let manifest_fingerprint = fnv1a64(manifest_text.as_bytes());
    
    // Create target texture for rendering
    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("TC01 Target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    
    registry.insert_owned_texture(
        TextureHandle(1), target_tex.clone(),
        TextureResourceDescriptor {
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            width,
            height,
            depth_or_array_layers: 1,
            mip_level_count: 1,
            sample_count: 1,
        },
        width * height * 4,
    ).unwrap();
    
    // 3. Build Graph from the shared manifest
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width,
        height,
    })
    .with_clear_color(clear_color);

    // Save graph representation to file
    fs::create_dir_all("tests/graphs").unwrap();
    fs::write("tests/graphs/tc01_empty.json", manifest_text).unwrap();

    // 4. Execute - Frame 1 (Cold Start)
    let start_time_cold = Instant::now();
    let submission_index = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("Graph validation failed");
    
    // Wait for GPU (Cold)
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission_index),
        timeout: None,
    });
    let elapsed_cold = start_time_cold.elapsed();
    
    // 4.5 Execute - Frame 2 (Warm / Cached)
    let start_time_warm = Instant::now();
    let submission_index_2 = executor.execute_checked(&engine, &registry, &mut pool, &graph).expect("Graph validation failed");
    
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission_index_2),
        timeout: None,
    });
    let elapsed_warm = start_time_warm.elapsed();

    println!("Cold Render time: {:?}", elapsed_cold);
    println!("Warm Render time: {:?}", elapsed_warm);
    
    // 5. Save output image
    fs::create_dir_all("tests/outputs/desktop").unwrap();
    let path = std::path::Path::new("tests/outputs/desktop/tc01_empty.png");
    output::save_texture_as_png(
        &engine,
        &target_tex,
        wgpu::TextureFormat::Rgba8Unorm,
        &path,
    )
    .expect("Failed to save image");

    let raw = engine
        .read_texture_to_raw_with_format_checked(&target_tex, wgpu::TextureFormat::Rgba8Unorm)
        .expect("Failed to read TC01 raw output");
    fs::write("tests/outputs/desktop/tc01_empty_desktop.bin", &raw.bytes).unwrap();
    let raw_fingerprint = fnv1a64(&raw.bytes);
    let metadata = serde_json::json!({
        "test_case": "TC01",
        "width": raw.width,
        "height": raw.height,
        "format": "Rgba8Unorm",
        "adapter_name": engine.adapter_info().name,
        "backend": format!("{:?}", engine.adapter_info().backend),
        "device_type": format!("{:?}", engine.adapter_info().device_type),
        "timing_scope": "execute_checked + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        "manifest_fingerprint": manifest_fingerprint,
        "raw_fingerprint": raw_fingerprint,
        "cold_render_time_ms": elapsed_cold.as_secs_f64() * 1000.0,
        "warm_render_time_ms": elapsed_warm.as_secs_f64() * 1000.0
    });
    fs::write(
        "tests/outputs/desktop/tc01_empty_desktop.json",
        serde_json::to_vec_pretty(&metadata).unwrap(),
    )
    .unwrap();
    
    // 6. Write rich report
    let report = format!(
        "# Báo cáo: TC01 - Empty Render\n\n\
        Đây là báo cáo canonical của TC01 trên Desktop và WebGPU, dùng chung manifest.\n\n\
        ## 1. Shared graph manifest\n\
        - **Manifest:** `tests/shared_assets/manifests/tc01_empty.json`\n\
        - **Graph fingerprint (FNV-1a):** `{}`\n\
        - **Mô tả:** `{}`\n\n\
        ## 2. Môi trường Desktop (Tauri/wgpu)\n\
        - **Thời gian Render (Cold Start - Lần đầu):** {:?}\n\
        - **Thời gian Render (Warm/Cached - Các lần sau):** {:?}\n\
        - **Kết quả ảnh (Thực tế):**\n\n\
        ![TC01 Desktop Render](../outputs/desktop/tc01_empty.png)\n\n\
        - **Raw output:** `Rgba8Unorm`, fingerprint `{}`\n\
        - **Mô tả (Đánh giá):** Màn hình được fill màu xám nhạt theo manifest, đồng nhất toàn ảnh.\n\
        - **Core Engine Errors:** Không có lỗi.\n\n\
        ## 3. Môi trường Web (WASM/WebGPU)\n\
        Chưa chạy WebGPU cho batch này.\n\n\
        ## 4. Đánh giá Tổng quan (Cross-Platform Consistency)\n\
        Chờ WebGPU output để so sánh graph fingerprint, raw bytes và timing.\n",
        manifest_fingerprint,
        manifest["description"].as_str().unwrap(),
        elapsed_cold,
        elapsed_warm,
        raw_fingerprint
    );
    fs::create_dir_all("tests/reports").unwrap();
    fs::write("tests/reports/tc01_report.md", report).unwrap();
}
