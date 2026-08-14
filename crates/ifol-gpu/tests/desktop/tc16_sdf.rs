mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SdfShapeUniform {
    shape_type: f32,
    size_x: f32,
    size_y: f32,
    corner_radius: f32,
    color: [f32; 4],
    border_color: [f32; 4],
    border_width: f32,
    glow_strength: f32,
    pos_x: f32,
    pos_y: f32,
    rotation: f32,
    scale: f32,
    aspect_ratio: f32,
    _pad: f32,
}

#[test]
fn run_tc16_sdf() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let tex_noise = h.load_texture("noise_perlin.jpeg"); // Just a dummy texture for BG0

        // Register Pipelines
        let pipe_sdf = h.register_pipeline(
            "sdf_shapes.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        let screen_aspect = 800.0f32 / 600.0f32;

        // 1. Circle (Red glowing sun)
        let circle_uni = SdfShapeUniform {
            shape_type: 0.0,
            size_x: 0.8,
            size_y: 0.8,
            corner_radius: 0.0,
            color: [0.9, 0.2, 0.3, 1.0], // Crimson Red
            border_color: [1.0, 0.6, 0.2, 1.0], // Orange Glow
            border_width: 0.05,
            glow_strength: 3.5,
            pos_x: -0.5,
            pos_y: 0.5,
            rotation: 0.0,
            scale: 0.3,
            aspect_ratio: screen_aspect,
            _pad: 0.0,
        };
        let bg_circle = h.create_custom_uniform_bind_group(circle_uni, "Circle Uniform");

        // 2. Rounded Rect (UI Card)
        let rect_uni = SdfShapeUniform {
            shape_type: 1.0,
            size_x: 0.7,
            size_y: 0.4,
            corner_radius: 0.15,
            color: [0.1, 0.5, 0.8, 0.9], // Blue Semi-transparent
            border_color: [0.6, 0.9, 1.0, 1.0], // Cyan outline
            border_width: 0.03,
            glow_strength: 0.0,
            pos_x: 0.5,
            pos_y: 0.5,
            rotation: 0.2, // Tilted
            scale: 0.4,
            aspect_ratio: screen_aspect,
            _pad: 0.0,
        };
        let bg_rect = h.create_custom_uniform_bind_group(rect_uni, "Rect Uniform");

        // 3. Ring (Target reticle)
        let ring_uni = SdfShapeUniform {
            shape_type: 2.0,
            size_x: 0.6,
            size_y: 0.6,
            corner_radius: 0.0,
            color: [0.0, 0.0, 0.0, 0.0], // Fill not used for ring
            border_color: [0.2, 0.9, 0.4, 1.0], // Neon Green
            border_width: 0.08,
            glow_strength: 4.0,
            pos_x: -0.5,
            pos_y: -0.4,
            rotation: 0.0,
            scale: 0.35,
            aspect_ratio: screen_aspect,
            _pad: 0.0,
        };
        let bg_ring = h.create_custom_uniform_bind_group(ring_uni, "Ring Uniform");

        // 4. Triangle (Play Button)
        let tri_uni = SdfShapeUniform {
            shape_type: 3.0,
            size_x: 0.6,
            size_y: 0.6,
            corner_radius: 0.0,
            color: [0.8, 0.1, 0.9, 1.0], // Magenta
            border_color: [1.0, 0.5, 1.0, 1.0],
            border_width: 0.04,
            glow_strength: 2.5,
            pos_x: 0.5,
            pos_y: -0.4,
            rotation: -1.5708, // Rotate to point right (Play button)
            scale: 0.3,
            aspect_ratio: screen_aspect,
            _pad: 0.0,
        };
        let bg_tri = h.create_custom_uniform_bind_group(tri_uni, "Tri Uniform");

        // Targets
        let (target_id, target_tex) = h.create_target("TC16 Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.08, 1.0]); // Dark slate background

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sdf, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_circle, Vec::new()),
                DrawCommand::new(pipe_sdf, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_rect, Vec::new()),
                DrawCommand::new(pipe_sdf, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_ring, Vec::new()),
                DrawCommand::new(pipe_sdf, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_tri, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC16 - 2D SDF Shapes & Vector Graphics",
            "features": [
                "Resolution-independent SDF rendering",
                "Smoothstep Anti-Aliasing",
                "Glowing borders and stroke thickness",
                "Aspect ratio preserved scaling"
            ],
            "shapes": ["Circle", "Rounded Rect", "Neon Ring", "Triangle"]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc16_sdf.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc16_sdf",
            "2D SDF Shapes & Vector Graphics",
            "Trình diễn 4 hình cơ bản UI dựng bằng kỹ thuật Signed Distance Field: Mặt trời đỏ (Circle), Thẻ giao diện (Rounded Rect), Vòng tròn Neon (Ring) và Nút Play (Triangle). Tất cả được bo viền sáng (glow) và khử răng cưa mượt mà.",
            "Xác thực năng lực dựng Vector Graphics bằng GPU (không cần Texture) của ifol-gpu.",
        );
    });
}
