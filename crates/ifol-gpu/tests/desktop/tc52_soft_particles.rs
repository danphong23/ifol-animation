mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SoftParticleUniform {
    pos: [f32; 2],
    scale: [f32; 2],
    particle_depth: f32,
    softness: f32,
    core_intensity: f32,
    _pad: f32,
    particle_color: [f32; 4],
}

#[test]
fn run_tc52_soft_particles() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        
        let screen_aspect = 800.0f32 / 600.0f32;
        let tex_heroes = h.load_texture("sprites_heroes.jpeg");
        let tex_scifi = h.load_texture("bg_scifi.jpeg");
        
        let pipe_chroma = h.register_pipeline("chroma_key_cropped.wgsl", Some(wgpu::BlendState::ALPHA_BLENDING), true, true);
        let pipe_particle = h.register_pipeline("soft_particle.wgsl", Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        }), true, true);

        // 1. Background (Z = 0.95)
        let bg_uniform = harness::SpriteUniform {
            pos: [0.0, 0.0],
            scale: [1.0, 1.0],
            uv_min: [0.0, 0.0],
            uv_max: [1.0, 1.0],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.95,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_scifi_uni = h.create_sprite_uniform_bind_group(bg_uniform);

        // 2. Paladin in center (Z = 0.50)
        let p_scale_y = 0.85f32;
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
            z_depth: 0.50,
            opacity: 1.0,
            _pad: 0.0,
        };
        let bg_paladin = h.create_sprite_uniform_bind_group(paladin_uni);

        // 3. Glowing Plasma Energy Sphere (Z = 0.48, intersecting Paladin's sword / chest)
        let particle_uni = SoftParticleUniform {
            pos: [0.10, 0.05],
            scale: [0.38, 0.38 * screen_aspect],
            particle_depth: 0.48,
            softness: 0.25,
            core_intensity: 2.5,
            _pad: 0.0,
            particle_color: [0.15, 0.85, 1.0, 0.90],
        };
        let bg_particle = h.create_custom_uniform_bind_group(particle_uni, "SoftParticle Uniform");

        let (final_target_id, final_target_tex) = h.create_target("Final Target");
        let (depth_target_id, _depth_tex) = h.create_depth_target("Depth Target");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: final_target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.0, 0.0, 0.0, 1.0]);

        graph.depth_stencil = Some(depth_target_id);

        graph.add_batch(
            &mut h.pool,
            vec![
                // 1. Background (Z = 0.95)
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_scifi.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_scifi_uni, Vec::new()),
                // 2. Paladin (Z = 0.50)
                DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_paladin, Vec::new()),
                // 3. Soft Plasma Sphere (Z = 0.48)
                DrawCommand::new(pipe_particle, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                    .with_bind_group(0, tex_heroes.bind_group.clone(), Vec::new())
                    .with_bind_group(1, bg_particle, Vec::new()),
            ],
        );

        h.executor.execute(&h.engine, &mut h.registry, &mut h.pool, &mut graph).expect("Execution failed");

        let graph_json = serde_json::json!({
            "test_case": "TC52 - Soft Particle Depth Fading & Volumetric Energy Sphere",
            "features": [
                "Depth Buffer Interaction with Geometry",
                "Volumetric Spherical Thickness Modeling",
                "Smooth Radial Falloff & Additive Plasma Blending"
            ]
        });
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc52_soft_particles.json", serde_json::to_string_pretty(&graph_json).unwrap()).unwrap();

        h.execute_and_record(
            &graph,
            &final_target_tex,
            "tc52_soft_particles",
            "Soft Particle Depth Fading",
            "Mô phỏng quả cầu năng lượng plasma (Volumetric Energy Sphere) bao bọc và giao thoa mềm mại với cơ thể Paladin mà không bị lỗi cắt phẳng đường viền (Hard Intersection Artifact) nhờ vào cấu trúc Falloff độ dày hình cầu và Depth Buffer.",
            "Xác thực sự tương tác giữa Depth Stencil Attachment và Shader hòa trộn quang học Additive.",
        );
    });
}
