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
struct ColorGradingUniform {
    params: [f32; 4],
    shadow_tint_vig: [f32; 4],
    highlight_tint: [f32; 4],
}

#[test]
fn run_tc14_grading() {
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
        let pipe_grading = h.register_pipeline("color_grading_filmic.wgsl", Some(wgpu::BlendState::REPLACE), false, true);
        let pipe_sparks = h.register_pipeline(
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

        // 3. Golden Hour Sunset Sky
        let sunset_sky_uni = SkyUniform {
            top_color: [0.35, 0.12, 0.25], // Sunset Crimson
            noise_strength: 0.04,
            bottom_color: [0.38, 0.25, 0.08], // Golden Amber
            time: 1.0,
        };
        let bg_sky = h.create_custom_uniform_bind_group(sunset_sky_uni, "Sunset Sky Uniform");

        // Background Trees
        let t1_scale_y = 0.65f32;
        let t1_crop_w = (0.18 - 0.01) * tex_forest.width as f32;
        let t1_crop_h = (0.42 - 0.01) * tex_forest.height as f32;
        let t1_scale_x = t1_scale_y * (t1_crop_w / t1_crop_h) * (1.0 / screen_aspect);
        let tree1_uni = harness::SpriteUniform {
            pos: [-0.60, 0.20],
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

        let t2_scale_y = 0.65f32;
        let t2_crop_w = (0.57 - 0.39) * tex_forest.width as f32;
        let t2_crop_h = (0.42 - 0.01) * tex_forest.height as f32;
        let t2_scale_x = t2_scale_y * (t2_crop_w / t2_crop_h) * (1.0 / screen_aspect);
        let tree2_uni = harness::SpriteUniform {
            pos: [0.60, 0.20],
            scale: [t2_scale_x, t2_scale_y],
            uv_min: [0.39, 0.01],
            uv_max: [0.57, 0.42],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.7,
            opacity: 0.95,
            _pad: 0.0,
        };
        let bg_tree2 = h.create_custom_uniform_bind_group(tree2_uni, "Tree 2 Uniform");

        // Paladin Girl (Left Hero)
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

        // Mage Boy (Right Hero)
        let m_scale_y = 0.58f32;
        let m_crop_w = (0.54 - 0.27) * tex_heroes.width as f32;
        let m_crop_h = (0.98 - 0.01) * tex_heroes.height as f32;
        let m_scale_x = m_scale_y * (m_crop_w / m_crop_h) * (1.0 / screen_aspect);
        let mage_uni = harness::SpriteUniform {
            pos: [0.35, -0.18],
            scale: [m_scale_x, m_scale_y],
            uv_min: [0.27, 0.01],
            uv_max: [0.54, 0.98],
            key_color: [0.0, 1.0, 0.0],
            tolerance: 0.48,
            smoothness: 0.10,
            z_depth: 0.3,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_mage = h.create_custom_uniform_bind_group(mage_uni, "Mage Uniform");

        // Golden Chest (Foreground)
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

        // 4. Color Grading Filmic Uniform: Golden Hour Split-Toning + ACES Curve
        let grade_uni = ColorGradingUniform {
            params: [0.12, 1.15, 1.25, 0.65], // exposure, contrast, saturation, temperature
            shadow_tint_vig: [0.08, 0.03, 0.18, 0.28], // shadow_r, shadow_g, shadow_b, vignette
            highlight_tint: [0.25, 0.16, 0.04, 0.0], // highlight_r, highlight_g, highlight_b, pad
        };
        let bg_grade = h.create_custom_uniform_bind_group(grade_uni, "Color Grading Uniform");

        // 5. Targets
        let (scene_target_id, _) = h.create_target("Scene Pass Target");
        let bg_scene_view = h.create_texture_bind_group(scene_target_id, "Scene View");

        let (final_target_id, final_target_tex) = h.create_target("Final Graded Target");

        // PASS 1: Render Scene
        let mut graph_scene = RenderGraph::new(RenderTarget::Offscreen {
            color: scene_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.20, 0.10, 0.15, 1.0]);

        graph_scene.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_sky, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_noise.bind_group, Vec::new())
                    .with_bind_group(1, bg_sky, Vec::new()),
                DrawCommand::new(pipe_sparks, DrawAction::Procedural { vertex_count: 6, instance_range: 0..40 })
                    .with_bind_group(0, tex_props.bind_group, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree1, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_forest.bind_group, Vec::new())
                    .with_bind_group(1, bg_tree2, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group, Vec::new())
                    .with_bind_group(1, bg_mage, Vec::new()),
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_items.bind_group, Vec::new())
                    .with_bind_group(1, bg_chest, Vec::new()),
            ],
        );

        // PASS 2: ACES Filmic Color Grading & Split-Toning Pass
        let mut graph_final = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph_final.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(pipe_grading, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, bg_scene_view, Vec::new())
                    .with_bind_group(1, bg_grade, Vec::new()),
            ],
        );

        // 6. Execute Passes
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_scene).expect("Scene pass failed");
        h.executor.execute_checked(&h.engine, &mut h.registry, &mut h.pool, &mut graph_final).expect("Grading pass failed");

        // 7. Serialize Graph JSON
        let graph_json = serde_json::json!({
            "test_case": "TC14 - Cinematic Color Grading & ACES Filmic Tone Mapping",
            "color_pipeline": {
                "tone_mapping": "ACES Filmic (Narkowicz Curve)",
                "color_temperature": "Golden Hour Warm (+0.65)",
                "split_toning": {
                    "shadows": "Deep Indigo Violet [0.08, 0.03, 0.18]",
                    "highlights": "Warm Golden Amber [0.25, 0.16, 0.04]"
                },
                "vignette": "Active (0.28 strength)"
            },
            "target": "Offscreen 800x600"
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc14_grading.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        // 8. Record Output
        h.execute_and_record(
            &graph_final,
            &final_target_tex,
            "tc14_grading",
            "Cinematic Color Grading & ACES Filmic Tone Mapping",
            "Khung cảnh hoàng hôn rực rỡ với đường cong ACES Filmic Tone Mapping, hiệu ứng Split-Toning hòa sắc bóng tím chàm và ánh sáng vàng hổ phách, kết hợp Vignette viền mềm tạo cảm giác điện ảnh đỉnh cao.",
            "Xác thực toàn diện Pipeline Color Grading & Tone Mapping hậu kỳ của ifol-gpu. Hoàn thành kiểm tra độ chuẩn xác xử lý dải màu động và phân loại màu sắc điện ảnh.",
        );
    });
}
