mod harness;

use harness::DesktopTestHarness;
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use serde_json::Value;
use std::fs;

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn array2(value: &Value) -> [f32; 2] {
    [
        value[0].as_f64().unwrap() as f32,
        value[1].as_f64().unwrap() as f32,
    ]
}

#[test]
fn run_tc03_zbuffer() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc03_zbuffer.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC03 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;
        let clear_color = [
            graph_spec["clear_color"][0].as_f64().unwrap() as f32,
            graph_spec["clear_color"][1].as_f64().unwrap() as f32,
            graph_spec["clear_color"][2].as_f64().unwrap() as f32,
            graph_spec["clear_color"][3].as_f64().unwrap() as f32,
        ];

        let pipe_id = h.register_pipeline(
            graph_spec["operations"][0]["shader"].as_str().unwrap(),
            Some(wgpu::BlendState::REPLACE),
            true,
            true,
        );
        let (target_id, target_tex) = h.create_target("TC03 Color Target");
        let (depth_id, _depth_tex) = h.create_depth_target("TC03 Depth Target");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width,
            height,
        })
        .with_clear_color(clear_color)
        .with_depth_stencil(depth_id);

        let mut commands = Vec::new();
        for operation in graph_spec["operations"].as_array().unwrap() {
            let texture = h.load_texture(operation["asset"].as_str().unwrap());
            let crop = operation["crop_uv"].as_array().unwrap();
            let uniform = h.build_sprite_uniform(
                &texture,
                array2(&operation["position"]),
                operation["target_height_scale"].as_f64().unwrap() as f32,
                [crop[0].as_f64().unwrap() as f32, crop[1].as_f64().unwrap() as f32],
                [crop[2].as_f64().unwrap() as f32, crop[3].as_f64().unwrap() as f32],
                operation["tolerance"].as_f64().unwrap() as f32,
                operation["smoothness"].as_f64().unwrap() as f32,
                operation["z_depth"].as_f64().unwrap() as f32,
                operation["opacity"].as_f64().unwrap() as f32,
            );
            let uniform_bind_group = h.create_sprite_uniform_bind_group(uniform);
            commands.push(
                DrawCommand::new(
                    pipe_id,
                    DrawAction::Procedural {
                        vertex_count: operation["vertex_count"].as_u64().unwrap() as u32,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, texture.bind_group, Vec::new())
                .with_bind_group(1, uniform_bind_group, Vec::new()),
            );
        }
        graph.add_batch(&mut h.pool, commands);

        fs::create_dir_all("tests/graphs").unwrap();
        fs::write("tests/graphs/tc03_zbuffer.json", manifest_text).unwrap();
        h.execute_and_record(
            &graph,
            &target_tex,
            "tc03_zbuffer",
            manifest["title"].as_str().unwrap(),
            manifest["description"].as_str().unwrap(),
            manifest["evaluation"]["visual_check"].as_str().unwrap(),
        );

        let metadata_path = "tests/outputs/desktop/tc03_zbuffer_desktop.json";
        let mut metadata: Value =
            serde_json::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        metadata["manifest_fingerprint"] = Value::String(fnv1a64(manifest_text.as_bytes()));
        fs::write(metadata_path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    });
}
