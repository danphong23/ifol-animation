mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct PixelationUniform {
    block_size: f32, // Size of pixels in screen coords
    screen_width: f32,
    screen_height: f32,
    _pad0: f32,
}

#[test]
fn run_tc33_pixelation() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_pixel = h.register_pipeline("pixelation.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

        // Paladin
        let p_scale_y = 0.80f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin");

        let pixel_uni = PixelationUniform {
            block_size: 16.0, // 16x16 pixel blocks
            screen_width: 800.0,
            screen_height: 600.0,
            _pad0: 0.0,
        };
        let bg_pixel_uni = h.create_custom_uniform_bind_group(pixel_uni, "Pixelation Uniform");

        let (target_a_id, _target_a_tex) = h.create_target("Target A");
        let (final_target_id, final_target_tex) = h.create_target("Final Target");

        let bg_tex_a = h.create_texture_bind_group(target_a_id, "Pixel Texture BG");

        // Pass 1: Extract Paladin via Chroma Key to transparent offscreen target
        let mut graph_chroma = RenderGraph::new(RenderTarget::Offscreen {
            color: target_a_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 0.0]);

        graph_chroma.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
            ],
        );

        // Pass 2: Apply Pixelation over dark blue background
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.1, 0.1, 0.3, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_pixel, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_tex_a, Vec::new())
                    .with_bind_group(1, bg_pixel_uni, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_chroma).expect("Execution failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC33 - Pixelation / Mosaic Filter",
            "features": [
                "Screen Coordinate Snapping",
                "Resolution Independent Block Size"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc33_pixelation.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc33_pixelation",
            "Pixelation / Mosaic",
            "Bộ lọc khảm điểm ảnh (Mosaic/Pixelation) biến đổi kết cấu thành các ô vuông. Ở TC này đang cấu hình Block Size = 16px.",
            "Xác thực khả năng bẻ cong UV bằng hàm Floor để lấy mẫu theo mảng thay vì mượt mà.",
        );
    });
}
