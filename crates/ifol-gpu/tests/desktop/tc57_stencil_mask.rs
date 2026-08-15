mod harness;
use harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;

#[test]
fn run_tc57_stencil_mask() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;

        let bg_tex = h.load_texture("bg_nightsky.jpeg");
        let heroes_tex = h.load_texture("sprites_heroes.jpeg");

        // 1. Pipeline A (Mask): Pure Stencil Write (Increment from 0 to 1, no color write)
        let mask_pipe = h.register_stencil_pipeline(
            "stencil_mask.wgsl",
            wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Always,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::IncrementClamp, // 0 -> 1
                },
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: !0,
                write_mask: !0,
            },
            wgpu::ColorWrites::empty(), // Do not draw color for the mask
        );

        // 2. Pipeline B (Content): Draw only where Stencil != 0 (inside mask portal)
        let content_pipe = h.register_stencil_pipeline(
            "chroma_key_cropped.wgsl",
            wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::NotEqual, // Default ref is 0, so NotEqual(0) means == 1
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: !0,
                write_mask: 0,
            },
            wgpu::ColorWrites::ALL,
        );

        // Uniforms for Background and Character inside portal
        let bg_uniform = h.build_sprite_uniform(&bg_tex, [0.0, 0.0], 1.0, [0.0, 0.0], [1.0, 1.0], 0.0, 0.0, 0.5, 1.0);
        let ubg_bg = h.create_sprite_uniform_bind_group(bg_uniform);

        let wizard_uniform = h.build_sprite_uniform(&heroes_tex, [0.0, -0.05], 0.65, [0.30, 0.0], [0.52, 1.0], 0.45, 0.12, 0.4, 1.0);
        let ubg_wizard = h.create_sprite_uniform_bind_group(wizard_uniform);

        // 3. Targets (Color + DepthStencil)
        let (target_id, target_tex) = h.create_target("TC57 Target");
        let (ds_id, _ds_tex) = h.create_depth_stencil_target("TC57 DS");

        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        }).with_clear_color([0.05, 0.05, 0.08, 1.0]);

        graph.depth_stencil = Some(ds_id);

        // Batch 1: Write Circular Mask into Stencil Buffer
        graph.add_batch(&mut h.pool, vec![
            DrawCommand::new(mask_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_bg, Vec::new())
        ]);

        // Batch 2: Render Night Sky + Wizard strictly constrained inside the Stencil Mask
        graph.add_batch(&mut h.pool, vec![
            DrawCommand::new(content_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, bg_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_bg, Vec::new()),
            DrawCommand::new(content_pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
                .with_bind_group(0, heroes_tex.bind_group, Vec::new())
                .with_bind_group(1, ubg_wizard, Vec::new()),
        ]);

        // Execute and record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc57_stencil_mask",
            "Hardware Stencil Buffer Masking & Portal Clipping",
            "Sử dụng Stencil State (IncrementClamp và NotEqual) để tạo mặt nạ hình tròn hoàn hảo ở tâm màn hình. Toàn bộ cảnh bầu trời đêm và Wizard chỉ hiển thị bên trong hình tròn Stencil, bên ngoài giữ nguyên màu nền đen vũ trụ.",
            "Mặt nạ tròn sắc nét 100% chuẩn tỷ lệ hình học, không bị méo ellipse, nhân vật Wizard đứng nổi bật giữa portal đêm mà không bị tràn ra ngoài."
        );

        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc57_stencil_mask.json", serde_json::json!({
            "test_case": "TC57 - Stencil Mask"
        }).to_string()).unwrap();
    });
}
