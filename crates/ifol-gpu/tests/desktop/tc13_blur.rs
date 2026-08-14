mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyUniform {
    top_color: [f32; 3],
    noise_strength: f32,
    bottom_color: [f32; 3],
    time: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlurUniform {
    direction: [f32; 2],
    radius: f32,
    _pad: f32,
}

#[test]
fn run_tc13_blur() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        // 1. Load Textures
        let tex_noise = h.load_texture("noise_perlin.jpeg");
        let tex_forest = h.load_texture("bg_forest_props1.jpeg");
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_items = h.load_texture("sprites_items.jpeg");
        let tex_props = h.load_texture("bg_nightsky_props.jpeg");

        // 2. Register Pipelines
        let pipe_sky = h.register_pipeline("sky_composite.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), false, true);
        let pipe_blur = h.register_pipeline("gaussian_blur_separable.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_blit = h.register_pipeline("texture_blit.wgsl", Some(wgpu::BlendState::REPLACE), false, false);
        let pipe_wisps = h.register_pipeline(
            "star_particles_sprite.wgsl",
            Some(wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            }),
            false,
            false,
        );

        let screen_aspect = 800.0f32 / 600.0f32;

        // 3. Background Forest Environment Props (To be blurred in DOF)
        let forest_sky_uni = SkyUniform {
            top_color: [0.03, 0.12, 0.18], // Enchanted forest teal
            noise_strength: 0.05,
            bottom_color: [0.02, 0.22, 0.08], // Deep moss green
            time: 1.0,
        };
        let bg_forest_sky = h.create_custom_uniform_bind_group(forest_sky_uni, "Forest Sky Uniform");

        // Distant Tree 1 (Left Oak)
        let t1_scale_y = 0.65f32;
        let t1_crop_w = (0.18 - 0.01) * tex_forest.width as f32;
        let t1_crop_h = (0.42 - 0.01) * tex_forest.height as f32;
        let t1_scale_x = t1_scale_y * (t1_crop_w / t1_crop_h) * (1.0 / screen_aspect);
        let tree1_uni = harness::SpriteUniform {
            pos: [-0.65, 0.25],
            scale: [t1_scale_x, t1_scale_y],
            uv_min: [0.01, 0.01],
            uv_max: [0.18, 0.42],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.7,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_tree1 = h.create_custom_uniform_bind_group(tree1_uni, "Tree 1 Uniform");

        // Distant Tree 2 (Center Pine)
        let t2_scale_y = 0.72f32;
        let t2_crop_w = (0.72 - 0.56) * tex_forest.width as f32;
        let t2_crop_h = (0.43 - 0.01) * tex_forest.height as f32;
        let t2_scale_x = t2_scale_y * (t2_crop_w / t2_crop_h) * (1.0 / screen_aspect);
        let tree2_uni = harness::SpriteUniform {
            pos: [0.0, 0.32],
            scale: [t2_scale_x, t2_scale_y],
            uv_min: [0.56, 0.01],
            uv_max: [0.72, 0.43],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.8,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_tree2 = h.create_custom_uniform_bind_group(tree2_uni, "Tree 2 Uniform");

        // Distant Tree 3 (Right Autumn Tree)
        let t3_scale_y = 0.65f32;
        let t3_crop_w = (0.57 - 0.39) * tex_forest.width as f32;
        let t3_crop_h = (0.42 - 0.01) * tex_forest.height as f32;
        let t3_scale_x = t3_scale_y * (t3_crop_w / t3_crop_h) * (1.0 / screen_aspect);
        let tree3_uni = harness::SpriteUniform {
            pos: [0.65, 0.25],
            scale: [t3_scale_x, t3_scale_y],
            uv_min: [0.39, 0.01],
            uv_max: [0.57, 0.42],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.7,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_tree3 = h.create_custom_uniform_bind_group(tree3_uni, "Tree 3 Uniform");

        // 4. Foreground Sharp Characters (Not Blurred)
        // Paladin Girl (Left Foreground)
        let p_scale_y = 0.58f32;
        let p_crop_w = (0.28 - 0.005) * tex_heroes.width as f32;
        let p_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let p_scale_x = p_scale_y * (p_crop_w / p_crop_h) * (1.0 / screen_aspect);
        let paladin_uni = harness::SpriteUniform {
            pos: [-0.35, -0.18],
            scale: [p_scale_x, p_scale_y],
            uv_min: [0.005, 0.01],
            uv_max: [0.28, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.3,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_custom_uniform_bind_group(paladin_uni, "Paladin Uniform");

        // Archer Girl (Right Foreground)
        let a_scale_y = 0.56f32;
        let a_crop_w = (0.80 - 0.54) * tex_heroes.width as f32;
        let a_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let a_scale_x = a_scale_y * (a_crop_w / a_crop_h) * (1.0 / screen_aspect);
        let archer_uni = harness::SpriteUniform {
            pos: [0.38, -0.15],
            scale: [a_scale_x, a_scale_y],
            uv_min: [0.54, 0.01],
            uv_max: [0.80, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.3,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_archer = h.create_custom_uniform_bind_group(archer_uni, "Archer Uniform");

        // Golden Chest (Foreground Center)
        let c_scale_y = 0.30f32;
        let c_crop_w = (0.42 - 0.10) * tex_items.width as f32;
        let c_crop_h = (0.95 - 0.52) * tex_items.height as f32;
        let c_scale_x = c_scale_y * (c_crop_w / c_crop_h) * (1.0 / screen_aspect);
        let chest_uni = harness::SpriteUniform {
            pos: [0.0, -0.62],
            scale: [c_scale_x, c_scale_y],
            uv_min: [0.10, 0.52],
            uv_max: [0.42, 0.95],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.2,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_chest = h.create_custom_uniform_bind_group(chest_uni, "Chest Uniform");

        // 5. Blur Uniforms (Horizontal & Vertical passes)
        let h_blur_uni = BlurUniform {
            direction: [1.0 / 800.0, 0.0],
            radius: 4.5,
            _pad: 0.0,
        };
        let bg_h_blur = h.create_custom_uniform_bind_group(h_blur_uni, "H-Blur Uniform");

        let v_blur_uni = BlurUniform {
            direction: [0.0, 1.0 / 600.0],
            radius: 4.5,
            _pad: 0.0,
        };
        let bg_v_blur = h.create_custom_uniform_bind_group(v_blur_uni, "V-Blur Uniform");

        // 6. Setup Ping-Pong Offscreen Targets
        let (target_a_id, _) = h.create_target("Target A (Background Scene)");
        let bg_target_a = h.create_texture_bind_group(target_a_id, "Target A View");

        let (target_b_id, _) = h.create_target("Target B (H-Blurred Ping-Pong)");
        let bg_target_b = h.create_texture_bind_group(target_b_id, "Target B View");

        let (final_target_id, final_target_tex) = h.create_target("Final Compositor Target");

        // PASS 1: Render Background Scene into Target A
        let mut graph_bg = RenderGraph::new(RenderTarget::Offscreen {
            color: target_a_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.02, 0.10, 0.15, 1.0]);

        graph_bg.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_forest_sky, Vec::new()),
                DrawCommand::new(pipe_wisps, DrawAction::Procedural { vertex_count: 6, instance_range: 0..40 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree1, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree2, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree3, Vec::new()),
            ],
        );

        // PASS 2: Horizontal Gaussian Blur (Target A -> Target B)
        let mut graph_h_blur = RenderGraph::new(RenderTarget::Offscreen {
            color: target_b_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_h_blur.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_blur, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_target_a, Vec::new())
                    .with_bind_group(1, bg_h_blur, Vec::new()),
            ],
        );

        // PASS 3: Vertical Gaussian Blur (Target B -> Target A Ping-Pong!)
        let mut graph_v_blur = RenderGraph::new(RenderTarget::Offscreen {
            color: target_a_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_v_blur.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_blur, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_target_b, Vec::new())
                    .with_bind_group(1, bg_v_blur, Vec::new()),
            ],
        );

        // PASS 4: Final Compositor (Blit Blurred Background + Render Crisp Foreground Heroes)
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                // Blit 2-Pass Blurred Background
                DrawCommand::new(pipe_blit, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_target_a, Vec::new()),

                // Sharp Foreground Characters
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_archer, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_chest, Vec::new()),
            ],
        );

        // 7. Execute Passes in Sequence
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_bg).expect("Pass 1 failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_h_blur).expect("Pass 2 failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_v_blur).expect("Pass 3 failed");
        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Pass 4 failed");

        // 8. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC13 - 2-Pass Gaussian Blur Filter & Cinematic Depth of Field",
            "passes": [
                "Pass 1: Render Background Forest Environment (Target A)",
                "Pass 2: Horizontal 9-Tap Gaussian Blur (Target A -> Target B)",
                "Pass 3: Vertical 9-Tap Gaussian Blur (Target B -> Target A Ping-Pong)",
                "Pass 4: Blit Blurred Background + Composite Razor-Sharp Foreground Heroes"
            ],
            "bokeh_depth_of_field": "Active",
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc13_blur.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 9. Record Final Output
        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc13_blur",
            "2-Pass Gaussian Blur Filter & Cinematic Depth of Field",
            "Khung cảnh hoàn chỉnh với kỹ thuật Depth of Field điện ảnh: Hậu cảnh Rừng thần thoại và Cây cối được làm mờ Gaussian 2-Pass mềm mại (Bokeh), trong khi Tiền cảnh với Nữ Hiệp Sĩ, Cung Thủ và Rương Vàng giữ độ sắc nét tuyệt đối.",
            "Xác thực cơ chế Ping-Pong Offscreen Render Targets và bộ lọc Separable Gaussian Blur 9-tap của ifol-gpu. Hoàn thành kiểm tra Depth of Field và đa Pass xử lý hậu kỳ.",
        );
    });
}
