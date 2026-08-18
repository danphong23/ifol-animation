use super::advanced_effects::record;
use super::harness::{DesktopTestHarness, SpriteUniform};
use ifol_gpu::graph::{DrawAction, DrawCommand, RenderGraph, RenderTarget};
use ifol_gpu::resources::{
    MeshHandle, MeshResourceDescriptor, PipelineHandle, PipelineLayoutResourceDescriptor,
};
use serde_json::Value;
use std::borrow::Cow;
use std::fs;
use std::path::Path;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BlendUniform {
    opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FlagVertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FlagUniform {
    time: f32,
    wave_freq: f32,
    wave_amp: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct KawaseUniform {
    offset: f32,
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
}

pub fn run_tc53() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc53_blend_modes.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let base = h.load_texture_exact("canonical_bg_scifi.png");
        let blend = h.load_texture_exact("canonical_sprites_heroes.png");
        let pipeline = h.register_dual_texture_pipeline(
            "blend_modes.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
        );
        let uniform = h.create_custom_uniform_bind_group(
            BlendUniform {
                opacity: 1.0,
                _pad0: 0.0,
                _pad1: 0.0,
                _pad2: 0.0,
            },
            "TC53 Blend Matrix",
        );
        let dual = h.create_dual_texture_bind_group(base.handle, blend.handle, "TC53 Dual Texture");
        let (target_id, target_texture) = h.create_target("TC53 Final");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipeline,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, dual, Vec::new())
                .with_bind_group(1, uniform, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_texture,
            "tc53_blend_modes",
            manifest_text,
            &manifest,
            "1 pass 4x2 blend matrix + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}

pub fn run_tc54() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc54_flag_mesh.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let grid_size = 32u32;
        let mut vertices = Vec::new();
        for y in 0..=grid_size {
            for x in 0..=grid_size {
                let u = x as f32 / grid_size as f32;
                let v = y as f32 / grid_size as f32;
                vertices.push(FlagVertex {
                    pos: [u * 1.4 - 0.7, (1.0 - v) - 0.5, 0.0],
                    uv: [u, v],
                });
            }
        }
        let mut indices = Vec::<u16>::new();
        for y in 0..grid_size {
            for x in 0..grid_size {
                let i0 = (y * (grid_size + 1) + x) as u16;
                let i1 = (y * (grid_size + 1) + x + 1) as u16;
                let i2 = ((y + 1) * (grid_size + 1) + x) as u16;
                let i3 = ((y + 1) * (grid_size + 1) + x + 1) as u16;
                indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
            }
        }
        let vertex_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC54 Flag Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        let index_buffer =
            h.engine
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("TC54 Flag Index Buffer"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
        let mesh_id = MeshHandle(54);
        h.registry
            .insert_mesh_with_descriptor(
                mesh_id,
                (
                    vertex_buffer,
                    Some((index_buffer, wgpu::IndexFormat::Uint16)),
                    indices.len() as u32,
                ),
                MeshResourceDescriptor {
                    vertex_buffer_size: (vertices.len() * std::mem::size_of::<FlagVertex>()) as u64,
                    vertex_count: vertices.len() as u32,
                    index_buffer_size: Some((indices.len() * std::mem::size_of::<u16>()) as u64),
                    index_format: Some(wgpu::IndexFormat::Uint16),
                },
            )
            .unwrap();
        let shader_code =
            fs::read_to_string(Path::new("tests/shared_assets/shaders/flag_mesh.wgsl")).unwrap();
        let shader = h
            .engine
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("TC54 Flag Shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_code)),
            });
        let layout = h
            .engine
            .device()
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("TC54 Flag Layout"),
                bind_group_layouts: &[Some(&h.texture_bg_layout), Some(&h.uniform_bg_layout)],
                immediate_size: 0,
            });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<FlagVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        };
        let pipeline = h
            .engine
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("TC54 Flag Pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Some(vertex_layout)],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            });
        let pipeline_id = PipelineHandle(54);
        h.registry.insert_pipeline_with_layout_descriptor(
            pipeline_id,
            pipeline,
            PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(1), Some(2)],
            },
        );
        let uniform = h.create_custom_uniform_bind_group(
            FlagUniform {
                time: 1.2,
                wave_freq: 6.0,
                wave_amp: 0.15,
                _pad: 0.0,
            },
            "TC54 Flag Uniform",
        );
        let (target_id, target_texture) = h.create_target("TC54 Final");
        let mut graph = RenderGraph::new(RenderTarget::Offscreen {
            color: target_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.08, 0.08, 0.12, 1.0]);
        graph.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    pipeline_id,
                    DrawAction::Indexed {
                        mesh: mesh_id,
                        index_range: 0..indices.len() as u32,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new())
                .with_bind_group(1, uniform, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&graph],
            &target_texture,
            "tc54_flag_mesh",
            manifest_text,
            &manifest,
            "1 pass indexed 32x32 mesh + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}

pub fn run_tc55() {
    let _ = env_logger::builder().is_test(true).try_init();
    pollster::block_on(async {
        let manifest_text = include_str!("../shared_assets/manifests/tc55_dual_kawase.json");
        let manifest: Value = serde_json::from_str(manifest_text).unwrap();
        let mut h = DesktopTestHarness::new(800, 600).await;
        let heroes = h.load_texture_exact("canonical_sprites_heroes.png");
        let scifi = h.load_texture_exact("canonical_bg_scifi.png");
        let chroma = h.register_pipeline(
            "chroma_key_cropped.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let kawase = h.register_pipeline(
            "dual_kawase.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            true,
        );
        let additive = h.register_pipeline(
            "texture_blit.wgsl",
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
        let screen = h.register_pipeline(
            "texture_blit.wgsl",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            false,
        );
        let aspect = 800.0 / 600.0;
        let crop_w = (0.54 - 0.27) * heroes.width as f32;
        let crop_h = (0.98 - 0.01) * heroes.height as f32;
        let mage = h.create_custom_uniform_bind_group(
            SpriteUniform {
                pos: [0.0, 0.0],
                scale: [0.8 * (crop_w / crop_h) / aspect, 0.8],
                uv_min: [0.27, 0.01],
                uv_max: [0.54, 0.98],
                key_color: [0.0, 1.0, 0.0],
                tolerance: 0.48,
                smoothness: 0.1,
                z_depth: 0.5,
                opacity: 1.0,
                _pad: 0.0,
            },
            "TC55 Mage",
        );
        let kawase_uniform = h.create_custom_uniform_bind_group(
            KawaseUniform {
                offset: 4.5,
                intensity: 2.2,
                _pad0: 0.0,
                _pad1: 0.0,
            },
            "TC55 Kawase Uniform",
        );
        let (mage_id, _) = h.create_target("TC55 Mage");
        let (down_id, _) = h.create_custom_target(400, 300, "TC55 Downsample");
        let (final_id, final_texture) = h.create_target("TC55 Final");
        let mage_bg = h.create_texture_bind_group(mage_id, "TC55 Mage Texture");
        let down_bg = h.create_texture_bind_group(down_id, "TC55 Down Texture");
        let mut extract = RenderGraph::new(RenderTarget::Offscreen {
            color: mage_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        extract.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    chroma,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, heroes.bind_group.clone(), Vec::new())
                .with_bind_group(1, mage.clone(), Vec::new()),
            ],
        );
        let mut down = RenderGraph::new(RenderTarget::Offscreen {
            color: down_id,
            width: 400,
            height: 300,
        })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
        down.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    kawase,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, mage_bg.clone(), Vec::new())
                .with_bind_group(1, kawase_uniform.clone(), Vec::new()),
            ],
        );
        let mut composite = RenderGraph::new(RenderTarget::Offscreen {
            color: final_id,
            width: 800,
            height: 600,
        })
        .with_clear_color([0.05, 0.05, 0.1, 1.0]);
        composite.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    screen,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, scifi.bind_group.clone(), Vec::new()),
            ],
        );
        composite.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    additive,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, down_bg, Vec::new()),
            ],
        );
        composite.add_batch(
            &mut h.pool,
            vec![
                DrawCommand::new(
                    screen,
                    DrawAction::Procedural {
                        vertex_count: 6,
                        instance_range: 0..1,
                    },
                )
                .with_bind_group(0, mage_bg, Vec::new()),
            ],
        );
        record(
            &mut h,
            &[&extract, &down, &composite],
            &final_texture,
            "tc55_dual_kawase",
            manifest_text,
            &manifest,
            "3 pass extract → 400x300 Kawase → composite + submit queue + device.poll(Wait); không gồm khởi tạo device/pipeline và readback",
        );
    });
}
