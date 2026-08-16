mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ReplaceUniform {
    transform: [[f32; 4]; 4],
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    target_hsv: [f32; 4],
    replace_hsv: [f32; 4],
    tolerance: f32,
    smoothness: f32,
    _pad: [f32; 2],
}

// Convert RGB to HSV for uniform setup
fn rgb2hsv(r: f32, g: f32, b: f32) -> [f32; 4] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let mut h = 0.0;
    if d > 0.0 {
        if max == r {
            h = (g - b) / d + (if g < b { 6.0 } else { 0.0 });
        } else if max == g {
            h = (b - r) / d + 2.0;
        } else {
            h = (r - g) / d + 4.0;
        }
        h /= 6.0;
    }
    let s = if max == 0.0 { 0.0 } else { d / max };
    let v = max;
    [h, s, v, 0.0]
}

#[test]
fn run_tc23_color_replace() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        // Use the heroes sprite sheet
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");

        let pipe_replace = h.register_pipeline(
            "color_replace.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true, // use sprite layout
        );

        // Scale y = 1.5. Scale x = 1.5 * (600/800) * (0.275 / 0.97) = 1.5 * 0.75 * 0.283 = 0.318
        let s_y = 1.5f32;
        let s_x = s_y * (600.0 / 800.0) * (0.275 / 0.97);

        // We want to replace the pink armor (around rgb 255, 180, 200) with a cool cyan armor (rgb 0, 200, 255)
        let target_hsv = rgb2hsv(255.0/255.0, 180.0/255.0, 200.0/255.0);
        let replace_hsv = rgb2hsv(0.0/255.0, 200.0/255.0, 255.0/255.0);

        let uniform = ReplaceUniform {
            transform: [
                [s_x, 0.0, 0.0, 0.0],
                [0.0, s_y, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            target_hsv,
            replace_hsv,
            tolerance: 0.35, // Wider tolerance
            smoothness: 0.2,
            _pad: [0.0; 2],
        };

        let bg_replace = h.create_custom_uniform_bind_group(uniform, "Replace Uniform");

        let (target_id, target_tex) = h.create_target("TC23 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_replace, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_replace, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC23 - Palette Swap (HSV Shift)",
            "features": [
                "Dynamic HSV based color replacement",
                "Preservation of value (shading/highlights)",
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc23_color_replace.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc23_color_replace",
            "Palette Swap (HSV Shift)",
            "Đổi màu giáp của nhân vật từ màu Hồng (Pink) sang màu Lục Lam (Cyan) dựa trên thuật toán HSV Shift.",
            "Test khả năng thay đổi màu sắc (Palette Swap) thời gian thực nhưng vẫn giữ nguyên khối (shading và highlight).",
        );
    });
}
