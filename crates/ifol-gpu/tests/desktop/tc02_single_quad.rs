mod harness;
use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use std::fs;
use serde_json::Value;

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[test]
fn run_tc02_single_quad() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let mut h = DesktopTestHarness::new(800, 600).await;
        let manifest_text = include_str!("../shared_assets/manifests/tc02_single_quad.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC02 shared manifest");
        let graph_spec = &manifest["graph"];
        let draw = &graph_spec["operations"][0];
        let target = &graph_spec["target"];
        let crop = draw["crop_uv"].as_array().unwrap();
        let position = draw["position"].as_array().unwrap();
        let clear = graph_spec["clear_color"].as_array().unwrap();
        let asset = draw["asset"].as_str().unwrap();
        let shader = draw["shader"].as_str().unwrap();

        // 1. Load texture from shared assets
        let tex_info = h.load_texture(asset);

        // 2. Crop Wizard (X: 30%..52%, Y: 0%..100%) with aspect ratio correction
        let wizard_uniform = h.build_sprite_uniform(
            &tex_info,
            [position[0].as_f64().unwrap() as f32, position[1].as_f64().unwrap() as f32],
            draw["target_height_scale"].as_f64().unwrap() as f32,
            [crop[0].as_f64().unwrap() as f32, crop[1].as_f64().unwrap() as f32],
            [crop[2].as_f64().unwrap() as f32, crop[3].as_f64().unwrap() as f32],
            draw["tolerance"].as_f64().unwrap() as f32,
            draw["smoothness"].as_f64().unwrap() as f32,
            draw["z_depth"].as_f64().unwrap() as f32,
            draw["opacity"].as_f64().unwrap() as f32,
        );
        let uniform_bg_id = h.create_sprite_uniform_bind_group(wizard_uniform);

        // 3. Register Pipeline with Chroma Key and Alpha Blending
        let pipe_id = h.register_pipeline(
            shader,
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );

        // 4. Create Target & Build Graph
        let (target_id, target_tex) = h.create_target("TC02 Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: target["width"].as_u64().unwrap() as u32,
            height: target["height"].as_u64().unwrap() as u32,
        })
        .with_clear_color([
            clear[0].as_f64().unwrap() as f32,
            clear[1].as_f64().unwrap() as f32,
            clear[2].as_f64().unwrap() as f32,
            clear[3].as_f64().unwrap() as f32,
        ]);

        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipe_id,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, tex_info.bind_group, Vec::new())
                .with_bind_group(1, uniform_bg_id, Vec::new()),
            ],
        );

        // 5. Serialize Graph JSON
        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc02_single_quad.json", manifest_text).unwrap();

        // 6. Execute & Record
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc02_single_quad",
            manifest["title"].as_str().unwrap(),
            manifest["description"].as_str().unwrap(),
            manifest["evaluation"]["visual_check"].as_str().unwrap(),
        );

        let metadata_path = "tests/outputs/desktop/tc02_single_quad_desktop.json";
        let mut metadata: Value = serde_json::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        metadata["manifest_fingerprint"] = Value::String(fnv1a64(manifest_text.as_bytes()));
        fs::write(metadata_path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    });
}
