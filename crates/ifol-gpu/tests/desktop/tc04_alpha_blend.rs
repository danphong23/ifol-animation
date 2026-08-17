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
fn run_tc04_alpha_blend() {
    let _ = env_logger::builder().is_test(true).try_init();

    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc04_alpha_blend.json");
        let manifest: Value = serde_json::from_str(manifest_text).expect("Invalid TC04 manifest");
        let graph_spec = &manifest["graph"];
        let target_spec = &graph_spec["target"];
        let width = target_spec["width"].as_u64().unwrap() as u32;
        let height = target_spec["height"].as_u64().unwrap() as u32;
        let mut h = DesktopTestHarness::new(width, height).await;

        let mut pipelines = std::collections::HashMap::new();
        for (pipeline_name, pipeline_spec) in graph_spec["pipelines"].as_object().unwrap() {
            let blend = match pipeline_spec["blend"].as_str().unwrap() {
                "Replace" => wgpu::BlendState::REPLACE,
                "AlphaBlend" => wgpu::BlendState::ALPHA_BLENDING,
                other => panic!("Unsupported TC04 blend mode: {other}"),
            };
            let shader = pipeline_spec["shader"].as_str().unwrap();
            pipelines.insert(
                pipeline_name.clone(),
                h.register_pipeline(shader, Some(blend), true, true),
            );
        }

        let clear_color = [
            graph_spec["clear_color"][0].as_f64().unwrap() as f32,
            graph_spec["clear_color"][1].as_f64().unwrap() as f32,
            graph_spec["clear_color"][2].as_f64().unwrap() as f32,
            graph_spec["clear_color"][3].as_f64().unwrap() as f32,
        ];
        let (target_id, target_tex) = h.create_target("TC04 Color Target");
        let (depth_id, _depth_tex) = h.create_depth_target("TC04 Depth Target");
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
                [
                    crop[0].as_f64().unwrap() as f32,
                    crop[1].as_f64().unwrap() as f32,
                ],
                [
                    crop[2].as_f64().unwrap() as f32,
                    crop[3].as_f64().unwrap() as f32,
                ],
                operation["tolerance"].as_f64().unwrap() as f32,
                operation["smoothness"].as_f64().unwrap() as f32,
                operation["z_depth"].as_f64().unwrap() as f32,
                operation["opacity"].as_f64().unwrap() as f32,
            );
            let uniform_bind_group = h.create_sprite_uniform_bind_group(uniform);
            let pipeline_name = operation["pipeline"].as_str().unwrap();
            let pipeline = *pipelines.get(pipeline_name).expect("TC04 pipeline missing");
            commands.push(
                DrawCommand::new(
                    pipeline,
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

        h.execute_and_record(
            &graph,
            &target_tex,
            "tc04_alpha_blend",
            manifest["title_vi"].as_str().unwrap(),
            manifest["description_vi"].as_str().unwrap(),
            manifest["evaluation"]["visual_check"].as_str().unwrap(),
        );

        let metadata_path = "tests/outputs/desktop/tc04_alpha_blend_desktop.json";
        let mut metadata: Value =
            serde_json::from_str(&fs::read_to_string(metadata_path).unwrap()).unwrap();
        metadata["manifest"] =
            Value::String("tests/shared_assets/manifests/tc04_alpha_blend.json".into());
        metadata["manifest_fingerprint"] = Value::String(fnv1a64(manifest_text.as_bytes()));
        fs::write(metadata_path, serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
    });
}
