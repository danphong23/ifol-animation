mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc59_sampler_modes() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let tex_info = h.load_texture("props_characters.jpg");

        // 1. Pipeline
        let pipe_id = h.register_pipeline(
            "sampler_modes.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 2. Create 3 Custom Samplers
        let s_repeat = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        
        let s_mirror = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::MirrorRepeat,
            address_mode_v: wgpu::AddressMode::MirrorRepeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        
        let s_clamp = h.engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Create texture bind groups using the 3 samplers manually
        let mut bg_counter = 1000;
        let bind_group_repeat = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &h.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&h.registry.texture(&tex_info.handle).unwrap().0) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&s_repeat) },
            ],
            label: Some("Repeat BG"),
        });
        let bg_repeat = ifol_gpu::resources::BindGroupHandle(bg_counter); bg_counter += 1;
        h.registry.insert_bind_group_with_descriptor(bg_repeat, bind_group_repeat, ifol_gpu::resources::BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1 }).unwrap();

        let bind_group_mirror = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &h.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&h.registry.texture(&tex_info.handle).unwrap().0) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&s_mirror) },
            ],
            label: Some("Mirror BG"),
        });
        let bg_mirror = ifol_gpu::resources::BindGroupHandle(bg_counter); bg_counter += 1;
        h.registry.insert_bind_group_with_descriptor(bg_mirror, bind_group_mirror, ifol_gpu::resources::BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1 }).unwrap();

        let bind_group_clamp = h.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &h.texture_bg_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&h.registry.texture(&tex_info.handle).unwrap().0) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&s_clamp) },
            ],
            label: Some("Clamp BG"),
        });
        let bg_clamp = ifol_gpu::resources::BindGroupHandle(bg_counter);
        h.registry.insert_bind_group_with_descriptor(bg_clamp, bind_group_clamp, ifol_gpu::resources::BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1 }).unwrap();

        // 3. Create Uniforms (pos, scale, expanded UV range [-0.5 .. 1.5])
        let u_repeat = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [-0.62, 0.0],
            scale: [0.26, 0.35],
            uv_min: [-0.5, -0.5],
            uv_max: [1.5, 1.5],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Uniform Repeat");
        
        let u_mirror = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.0, 0.0],
            scale: [0.26, 0.35],
            uv_min: [-0.5, -0.5],
            uv_max: [1.5, 1.5],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Uniform Mirror");

        let u_clamp = h.create_custom_uniform_bind_group(SpriteUniform {
            pos: [0.62, 0.0],
            scale: [0.26, 0.35],
            uv_min: [-0.5, -0.5],
            uv_max: [1.5, 1.5],
            key_color: [0.0, 0.0, 0.0],
            tolerance: 0.0,
            smoothness: 0.0,
            z_depth: 0.5,
            opacity: 1.0,
            _pad: 0.0,
        }, "Uniform Clamp");

        // 4. Graph
        let (target_id, target_tex) = h.create_target("TC59 Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.06, 0.06, 0.09, 1.0]);

        graph.add_batch(&mut h.pool, vec![
            DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_repeat, Vec::new())
                .with_bind_group(1, u_repeat, Vec::new()),
            DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_mirror, Vec::new())
                .with_bind_group(1, u_mirror, Vec::new()),
            DrawCommand::new(pipe_id, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_clamp, Vec::new())
                .with_bind_group(1, u_clamp, Vec::new()),
        ]);

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc59_sampler_modes",
            "Sampler Address Modes (Repeat, MirrorRepeat, ClampToEdge)",
            "Kiểm chứng 3 chế độ quấn Texture (Texture Wrapping Modes) của phần cứng GPU khi UV vượt ra ngoài khoảng [0, 1] (từ -0.5 đến 1.5). Khung 1: Lặp lại liên tục (Repeat); Khung 2: Lặp đối xứng gương (MirrorRepeat); Khung 3: Kẹp mép kéo dài viền (ClampToEdge).",
            "Ba khung hình chữ nhật hiển thị sắc nét song song với nhau: Khung trái tạo lưới lặp 2x2 liền mạch, khung giữa phản chiếu đối xứng hoàn hảo, khung phải kéo dài viền pixel ra 4 cạnh mà không gây sọc xé."
        );

        fs::write("tests/graphs/tc59_sampler_modes.json", serde_json::json!({"test_case": "TC59 - Sampler Modes"}).to_string()).unwrap();
    });
}
