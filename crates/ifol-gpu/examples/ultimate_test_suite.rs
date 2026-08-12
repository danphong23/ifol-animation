use std::borrow::Cow;
use std::time::Instant;
use ifol_gpu::api::{GpuEngineBuilder, GpuEngine};
use ifol_gpu::render::{
    DrawAction, DrawCommand, PipelineHandle, RenderGraph, RenderGraphExecutor, RenderTarget,
    ResourceRegistry, TextureHandle, RenderNodePool,
};
use ifol_gpu::render::handle::BindGroupHandle;
use image::GenericImageView;

struct TestHarness<'a> {
    engine: GpuEngine<'a>,
    executor: RenderGraphExecutor,
    registry: ResourceRegistry,
    pool: RenderNodePool,
    width: u32,
    height: u32,
    next_tex_id: u64,
    next_pipe_id: u64,
    next_bg_id: u64,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl<'a> TestHarness<'a> {
    async fn new(width: u32, height: u32) -> Self {
        let engine = GpuEngineBuilder::new().build().await.expect("Failed to build engine");
        
        let texture_bind_group_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        Self {
            engine,
            executor: RenderGraphExecutor::new(),
            registry: ResourceRegistry::new(),
            pool: RenderNodePool::new(),
            width,
            height,
            next_tex_id: 1,
            next_pipe_id: 1,
            next_bg_id: 1,
            texture_bind_group_layout,
            sampler,
        }
    }

    fn create_target(&mut self, label: &str) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.textures.insert(id, view);
        (id, tex)
    }

    fn create_depth_target(&mut self, label: &str) -> (TextureHandle, wgpu::Texture) {
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.textures.insert(id, view);
        (id, tex)
    }

    fn load_texture_and_bg(&mut self, path: &str) -> BindGroupHandle {
        let img = image::open(path).expect("Failed to open image");
        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();

        let size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
        let tex = self.engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(path),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.engine.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * w),
                rows_per_image: Some(h),
            },
            size,
        );

        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let tex_id = TextureHandle(self.next_tex_id);
        self.next_tex_id += 1;
        self.registry.textures.insert(tex_id, view);
        
        // Create BindGroup
        let view_ref = self.registry.textures.get(&tex_id).unwrap();
        let bind_group = self.engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view_ref),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
            label: Some(path),
        });

        let bg_id = BindGroupHandle(self.next_bg_id);
        self.next_bg_id += 1;
        self.registry.bind_groups.insert(bg_id, bind_group);
        bg_id
    }

    fn save_texture(&self, texture: &wgpu::Texture, filename: &str) {
        let path = std::path::Path::new("examples/outputs").join(filename);
        std::fs::create_dir_all("examples/outputs").unwrap();
        self.engine.save_texture_to_file(texture, &path).expect("Lỗi lưu ảnh");
        println!("Saved output to {:?}", path);
    }

    fn register_pipeline(&mut self, shader_code: &str, blend: Option<wgpu::BlendState>, depth: bool, uses_texture: bool) -> PipelineHandle {
        let shader = self.engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_code)),
        });

        let mut bgls: Vec<Option<&wgpu::BindGroupLayout>> = Vec::new();
        if uses_texture {
            bgls.push(Some(&self.texture_bind_group_layout));
        }

        let layout = self.engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &bgls,
            immediate_size: 0,
        });

        let pipe = self.engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: if depth {
                Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                })
            } else {
                None
            },
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        let id = PipelineHandle(self.next_pipe_id);
        self.next_pipe_id += 1;
        self.registry.pipelines.insert(id, pipe);
        id
    }
}

// Shader for simple quads
const BASIC_SHADER: &str = "
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};
@vertex fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5), vec2<f32>(-0.5,  0.5),
        vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5)
    );
    
    // instance 0: Red (Z=0.1), instance 1: Green (Z=0.5), instance 2: Blue (Z=0.9)
    let offset = f32(ii) * 0.2 - 0.2;
    let z = 0.1 + f32(ii) * 0.4;
    out.clip_position = vec4<f32>(pos[vi].x + offset, pos[vi].y + offset, z, 1.0);
    
    if (ii == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); }
    else if (ii == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); }
    else { out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
";

const BASIC_SHADER_LEFT: &str = "
struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
@vertex fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2<f32>, 6>(vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5), vec2<f32>(-0.5,  0.5), vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5));
    let offset = f32(ii) * 0.2 - 0.2; let z = 0.1 + f32(ii) * 0.4;
    out.clip_position = vec4<f32>(pos[vi].x + offset + 0.5, pos[vi].y + offset, z, 1.0); // Offset Left (so world moves right)
    if (ii == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); } else if (ii == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); } else { out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
";

const BASIC_SHADER_RIGHT: &str = "
struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
@vertex fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2<f32>, 6>(vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5), vec2<f32>(-0.5,  0.5), vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5));
    let offset = f32(ii) * 0.2 - 0.2; let z = 0.1 + f32(ii) * 0.4;
    out.clip_position = vec4<f32>(pos[vi].x + offset - 0.5, pos[vi].y + offset, z, 1.0); // Offset Right
    if (ii == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); } else if (ii == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); } else { out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
";

const ALPHA_SHADER: &str = "
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};
@vertex fn vs_main(@builtin(vertex_index) vi: u32, @builtin(instance_index) ii: u32) -> VertexOutput {
    var out: VertexOutput;
    var pos = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5), vec2<f32>(-0.5,  0.5),
        vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5)
    );
    
    let offset = f32(ii) * 0.2;
    let z = 0.5 - f32(ii) * 0.1; // ii=0 (opaque) is at 0.5, ii=1 (transparent) is at 0.4 (closer)
    out.clip_position = vec4<f32>(pos[vi].x + offset, pos[vi].y - offset, z, 1.0);
    
    if (ii == 0u) { out.color = vec4<f32>(0.5, 0.5, 0.5, 1.0); } // Opaque grey
    else { out.color = vec4<f32>(1.0, 1.0, 0.0, 0.5); } // Transparent yellow
    return out;
}
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
";

fn run_tc01_empty(harness: &mut TestHarness) {
    let (target, tex) = harness.create_target("tc01");
    let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: target, width: harness.width, height: harness.height })
        .with_clear_color([0.2, 0.2, 0.2, 1.0]);
    
    let start = Instant::now();
    let idx = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &graph);
    println!("TC01 Compile & Execute: {:?}", start.elapsed());
    
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    harness.save_texture(&tex, "tc01_empty.png");
}

fn run_tc02_tc03(harness: &mut TestHarness) {
    let pipe_basic = harness.register_pipeline(BASIC_SHADER, Some(wgpu::BlendState::REPLACE), true, false);
    
    // TC02: Single quad
    let (t02, tex02) = harness.create_target("tc02");
    let (z02, _) = harness.create_depth_target("z02");
    let mut g02 = RenderGraph::new(RenderTarget::Offscreen { color: t02, width: harness.width, height: harness.height })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]).with_depth_stencil(z02);
    g02.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })]);
    
    // TC03: Z-Buffer overlapping
    let (t03, tex03) = harness.create_target("tc03");
    let (z03, _) = harness.create_depth_target("z03");
    let mut g03 = RenderGraph::new(RenderTarget::Offscreen { color: t03, width: harness.width, height: harness.height })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]).with_depth_stencil(z03);
    g03.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..3 })]);

    let _ = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g02);
    let idx2 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g03);
    
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx2), timeout: None });
    harness.save_texture(&tex02, "tc02_single_quad.png");
    harness.save_texture(&tex03, "tc03_zbuffer.png");
}

fn run_tc04_alpha(harness: &mut TestHarness) {
    let pipe_alpha = harness.register_pipeline(ALPHA_SHADER, Some(wgpu::BlendState::ALPHA_BLENDING), true, false);
    let (t04, tex04) = harness.create_target("tc04");
    let (z04, _) = harness.create_depth_target("z04");
    
    let mut g04 = RenderGraph::new(RenderTarget::Offscreen { color: t04, width: harness.width, height: harness.height })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]).with_depth_stencil(z04);
    
    g04.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_alpha, DrawAction::Procedural { vertex_count: 6, instance_range: 0..2 })]);
    
    let idx = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g04);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    harness.save_texture(&tex04, "tc04_alpha_blend.png");
}

fn run_tc08_massive(harness: &mut TestHarness) {
    let pipe = harness.register_pipeline(BASIC_SHADER, Some(wgpu::BlendState::REPLACE), false, false);
    let (t08_1, tex08_1) = harness.create_target("tc08_1");
    let (t08_2, tex08_2) = harness.create_target("tc08_2");

    // Subgraph 1: 1 Node, 10,000 Commands
    let mut g1 = RenderGraph::new(RenderTarget::Offscreen { color: t08_1, width: harness.width, height: harness.height });
    let mut cmds = Vec::with_capacity(10000);
    for _ in 0..10000 {
        cmds.push(DrawCommand::new(pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }));
    }
    g1.add_batch(&mut harness.pool, cmds);

    // Subgraph 2: 10,000 Nodes, 1 Command each
    let mut g2 = RenderGraph::new(RenderTarget::Offscreen { color: t08_2, width: harness.width, height: harness.height });
    for _ in 0..10000 {
        g2.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })]);
    }

    let start = Instant::now();
    let _ = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g1);
    println!("TC08 (1 Node x 10000 Cmds) Compile Time: {:?}", start.elapsed());

    let start2 = Instant::now();
    let idx2 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g2);
    println!("TC08 (10000 Nodes x 1 Cmd) Compile Time: {:?}", start2.elapsed());

    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx2), timeout: None });
    harness.save_texture(&tex08_1, "tc08_massive_1.png");
    harness.save_texture(&tex08_2, "tc08_massive_2.png");
}

fn run_tc12_chroma_and_tc20_anime(harness: &mut TestHarness) {
    // 1. Load images and get bind groups
    let bg_bg = harness.load_texture_and_bg("examples/assets/images/anime_bg_nightsky.jpg");
    let char_bg = harness.load_texture_and_bg("examples/assets/images/anime_char_greenscreen.jpg");

    // 2. Load Shaders
    let chroma_wgsl = std::fs::read_to_string("examples/assets/shaders/chroma_key.wgsl").unwrap_or_default();
    
    let tex_shader = "
struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32>, };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(vec2<f32>(-1.0, -1.0), vec2<f32>( 1.0, -1.0), vec2<f32>(-1.0,  1.0), vec2<f32>(-1.0,  1.0), vec2<f32>( 1.0, -1.0), vec2<f32>( 1.0,  1.0));
    var out: VertexOutput; out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0); out.uv = pos[vi] * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5); return out;
}
@group(0) @binding(0) var t: texture_2d<f32>; @group(0) @binding(1) var s: sampler;
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return textureSample(t, s, in.uv); }
";

    // 3. Register Pipelines (uses_texture = true)
    let pipe_tex = harness.register_pipeline(tex_shader, Some(wgpu::BlendState::REPLACE), false, true);
    let pipe_chroma = harness.register_pipeline(&chroma_wgsl, Some(wgpu::BlendState::ALPHA_BLENDING), false, true);

    if chroma_wgsl.is_empty() { return; } // Skip if not found during stub testing

    // TC12: Chroma Key ONLY
    let (t12, tex12) = harness.create_target("tc12");
    let mut g12 = RenderGraph::new(RenderTarget::Offscreen { color: t12, width: harness.width, height: harness.height })
        .with_clear_color([0.0, 0.0, 0.0, 0.0]);
    g12.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, char_bg, vec![])]);
    let idx12 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g12);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx12), timeout: None });
    harness.save_texture(&tex12, "tc12_chroma.png");

    // TC20: Anime Scene (BG + Char)
    let (t20, tex20) = harness.create_target("tc20");
    let mut g20 = RenderGraph::new(RenderTarget::Offscreen { color: t20, width: harness.width, height: harness.height });
    
    // Node 1: Background
    g20.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_tex, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, bg_bg, vec![])]);
    // Node 2: Character (Chroma Keyed, Blended)
    g20.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_chroma, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, char_bg, vec![])]);

    let idx20 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g20);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx20), timeout: None });
    harness.save_texture(&tex20, "tc20_anime.png");
}

fn run_tc05_to_tc11_structure(harness: &mut TestHarness) {
    let pipe_basic = harness.register_pipeline(BASIC_SHADER, Some(wgpu::BlendState::REPLACE), false, false);

    // TC05 - Interleaved Passes
    let (t05, tex05) = harness.create_target("tc05");
    let mut g05 = RenderGraph::new(RenderTarget::Offscreen { color: t05, width: harness.width, height: harness.height })
        .with_clear_color([0.1, 0.1, 0.1, 1.0]);
    let sub_a = RenderGraph::new(RenderTarget::Offscreen { color: t05, width: harness.width, height: harness.height });
    let sub_b = RenderGraph::new(RenderTarget::Offscreen { color: t05, width: harness.width, height: harness.height });
    
    // add_subgraph automatically adds node_id
    g05.add_subgraph(&mut harness.pool, "A", sub_a, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })]);
    g05.add_subgraph(&mut harness.pool, "B", sub_b, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 1..2 })]);
    let idx05 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g05);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx05), timeout: None });
    harness.save_texture(&tex05, "tc05_interleaved.png");

    // TC06 & TC07 - GC and Deep Recursion
    let (t07, tex07) = harness.create_target("tc07");
    let mut g07 = RenderGraph::new(RenderTarget::Offscreen { color: t07, width: harness.width, height: harness.height })
        .with_clear_color([0.2, 0.3, 0.4, 1.0]);
    
    let sub1 = RenderGraph::new(RenderTarget::Offscreen { color: t07, width: harness.width, height: harness.height });
    let sub2 = RenderGraph::new(RenderTarget::Offscreen { color: t07, width: harness.width, height: harness.height });
    let sub3 = RenderGraph::new(RenderTarget::Offscreen { color: t07, width: harness.width, height: harness.height });
    
    let id3 = harness.pool.alloc_subgraph("Level3", sub3, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 2..3 })]);
    let mut sub2_modified = sub2.clone(); sub2_modified.add_node_id(id3);
    let id2 = harness.pool.alloc_subgraph("Level2", sub2_modified, vec![]);
    let mut sub1_modified = sub1.clone(); sub1_modified.add_node_id(id2);
    
    g07.add_subgraph(&mut harness.pool, "Level1", sub1_modified, vec![]);
    let idx07 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g07);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx07), timeout: None });
    harness.save_texture(&tex07, "tc07_recursion.png");

    // TC09 - Pipeline Caching (measure twice)
    let (t09, _) = harness.create_target("tc09");
    let mut g09 = RenderGraph::new(RenderTarget::Offscreen { color: t09, width: harness.width, height: harness.height });
    g09.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })]);
    
    let start1 = Instant::now();
    let _idx09_1 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g09);
    println!("TC09 First Compile: {:?}", start1.elapsed());
    
    let start2 = Instant::now();
    let idx09_2 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g09);
    println!("TC09 Second Compile (Cached): {:?}", start2.elapsed());
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx09_2), timeout: None });

    // TC10 - Missing Resource
    let (t10, _tex10) = harness.create_target("tc10");
    let mut g10 = RenderGraph::new(RenderTarget::Offscreen { color: t10, width: harness.width, height: harness.height })
        .with_clear_color([0.5, 0.0, 0.5, 1.0]); // Magenta background to indicate missing
    
    // Deliberately passing a missing bind group for a pipeline that uses it
    let tex_shader = "
struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) uv: vec2<f32>, };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 6>(vec2<f32>(-0.5, -0.5), vec2<f32>( 0.5, -0.5), vec2<f32>(-0.5,  0.5), vec2<f32>(-0.5,  0.5), vec2<f32>( 0.5, -0.5), vec2<f32>( 0.5,  0.5));
    var out: VertexOutput; out.clip_position = vec4<f32>(pos[vi], 0.0, 1.0); out.uv = pos[vi] * vec2<f32>(0.5, -0.5) + 0.5; return out;
}
@group(0) @binding(0) var t: texture_2d<f32>; @group(0) @binding(1) var s: sampler;
@fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return textureSample(t, s, in.uv); }
";
    let pipe_missing = harness.register_pipeline(tex_shader, Some(wgpu::BlendState::REPLACE), false, true);
    
    g10.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_missing, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })
        .with_bind_group(0, BindGroupHandle(9999), vec![])]); // Missing BG!
        
    // let idx10 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g10);
    // let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx10), timeout: None });
    // harness.save_texture(&tex10, "tc10_missing_resource.png");

    // let idx10 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g10);
    // let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx10), timeout: None });
    // harness.save_texture(&tex10, "tc10_missing_resource.png");
}

fn run_tc11_camera_benchmarks(harness: &mut TestHarness) {
    let pipe_basic = harness.register_pipeline(BASIC_SHADER, Some(wgpu::BlendState::REPLACE), false, false);
    let pipe_left = harness.register_pipeline(BASIC_SHADER_LEFT, Some(wgpu::BlendState::REPLACE), false, false);
    let pipe_right = harness.register_pipeline(BASIC_SHADER_RIGHT, Some(wgpu::BlendState::REPLACE), false, false);

    // TC11_A - 2 Different Scenes
    let (t11_a1, tex11_a1) = harness.create_target("tc11_a1");
    let (t11_a2, tex11_a2) = harness.create_target("tc11_a2");
    let mut g11_a1 = RenderGraph::new(RenderTarget::Offscreen { color: t11_a1, width: harness.width, height: harness.height }).with_clear_color([1.0, 0.0, 0.0, 1.0]);
    g11_a1.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })]);
    let mut g11_a2 = RenderGraph::new(RenderTarget::Offscreen { color: t11_a2, width: harness.width, height: harness.height }).with_clear_color([0.0, 0.0, 1.0, 1.0]);
    g11_a2.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 1..2 })]);
    
    let _ = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_a1);
    let idx_a = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_a2);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_a), timeout: None });
    harness.save_texture(&tex11_a1, "tc11_a_diff_scene1.png");
    harness.save_texture(&tex11_a2, "tc11_a_diff_scene2.png");

    // TC11_B - Same Scene, Different Cameras
    let (t11_b1, tex11_b1) = harness.create_target("tc11_b1");
    let (t11_b2, tex11_b2) = harness.create_target("tc11_b2");
    let mut g11_b1 = RenderGraph::new(RenderTarget::Offscreen { color: t11_b1, width: harness.width, height: harness.height }).with_clear_color([0.1, 0.1, 0.1, 1.0]);
    g11_b1.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_left, DrawAction::Procedural { vertex_count: 6, instance_range: 0..3 })]);
    let mut g11_b2 = RenderGraph::new(RenderTarget::Offscreen { color: t11_b2, width: harness.width, height: harness.height }).with_clear_color([0.1, 0.1, 0.1, 1.0]);
    g11_b2.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_right, DrawAction::Procedural { vertex_count: 6, instance_range: 0..3 })]);
    
    let _ = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_b1);
    let idx_b = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_b2);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_b), timeout: None });
    harness.save_texture(&tex11_b1, "tc11_b_cam_left.png");
    harness.save_texture(&tex11_b2, "tc11_b_cam_right.png");

    // TC11_E - Complex Benchmark Overlap
    let (t11_e1, tex11_e1) = harness.create_target("tc11_e1");
    let (t11_e2, tex11_e2) = harness.create_target("tc11_e2");
    let mut g11_e1 = RenderGraph::new(RenderTarget::Offscreen { color: t11_e1, width: harness.width, height: harness.height });
    let mut g11_e2 = RenderGraph::new(RenderTarget::Offscreen { color: t11_e2, width: harness.width, height: harness.height });
    
    // Push 5000 commands
    let mut cmds = Vec::with_capacity(5000);
    for _ in 0..5000 { cmds.push(DrawCommand::new(pipe_basic, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 })); }
    g11_e1.add_batch(&mut harness.pool, cmds.clone());
    g11_e2.add_batch(&mut harness.pool, cmds);
    
    let start = Instant::now();
    let _ = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_e1);
    println!("TC11_E (5000 Cmds) Compile Cam 1: {:?}", start.elapsed());
    
    let start2 = Instant::now();
    let idx_e = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g11_e2);
    println!("TC11_E (5000 Cmds) Compile Cam 2: {:?}", start2.elapsed());
    
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_e), timeout: None });
    harness.save_texture(&tex11_e1, "tc11_e_overlap1.png");
    harness.save_texture(&tex11_e2, "tc11_e_overlap2.png");
}

fn run_tc13_to_tc19_effects(harness: &mut TestHarness) {
    let bg_bg = harness.load_texture_and_bg("examples/assets/images/anime_bg_nightsky.jpg");
    
    let blur_wgsl = std::fs::read_to_string("examples/assets/shaders/blur.wgsl").unwrap_or_default();
    let snow_wgsl = std::fs::read_to_string("examples/assets/shaders/snow_particle.wgsl").unwrap_or_default();
    let disp_wgsl = std::fs::read_to_string("examples/assets/shaders/displacement.wgsl").unwrap_or_default();
    
    if blur_wgsl.is_empty() { return; }
    
    let pipe_blur = harness.register_pipeline(&blur_wgsl, Some(wgpu::BlendState::REPLACE), false, true);
    let pipe_snow = harness.register_pipeline(&snow_wgsl, Some(wgpu::BlendState::ALPHA_BLENDING), false, false);
    let pipe_disp = harness.register_pipeline(&disp_wgsl, Some(wgpu::BlendState::REPLACE), false, true);

    // TC13 & TC14 - Blur & Bloom (Using Blur Shader)
    let (t13, tex13) = harness.create_target("tc13");
    let mut g13 = RenderGraph::new(RenderTarget::Offscreen { color: t13, width: harness.width, height: harness.height });
    g13.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_blur, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, bg_bg, vec![])]);
    let idx13 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g13);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx13), timeout: None });
    harness.save_texture(&tex13, "tc13_blur.png");

    // TC15 - Snow Particles (Instancing)
    let (t15, tex15) = harness.create_target("tc15");
    let mut g15 = RenderGraph::new(RenderTarget::Offscreen { color: t15, width: harness.width, height: harness.height })
        .with_clear_color([0.0, 0.0, 0.0, 1.0]);
    let start_snow = Instant::now();
    g15.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_snow, DrawAction::Procedural { vertex_count: 6, instance_range: 0..50000 })]);
    let idx15 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g15);
    println!("TC15 Snow (50000 instances) Compile Time: {:?}", start_snow.elapsed());
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx15), timeout: None });
    harness.save_texture(&tex15, "tc15_snow.png");

    // TC16 - UV Displacement
    let (t16, tex16) = harness.create_target("tc16");
    let mut g16 = RenderGraph::new(RenderTarget::Offscreen { color: t16, width: harness.width, height: harness.height });
    g16.add_batch(&mut harness.pool, vec![DrawCommand::new(pipe_disp, DrawAction::Procedural { vertex_count: 6, instance_range: 0..1 }).with_bind_group(0, bg_bg, vec![])]);
    let idx16 = harness.executor.execute(&harness.engine, &harness.registry, &mut harness.pool, &g16);
    let _ = harness.engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx16), timeout: None });
    harness.save_texture(&tex16, "tc16_displacement.png");
}

fn main() {
    println!("Starting Ultimate Test Suite...");
    let mut harness = pollster::block_on(TestHarness::new(800, 600));
    
    run_tc01_empty(&mut harness);
    run_tc02_tc03(&mut harness);
    run_tc04_alpha(&mut harness);
    run_tc05_to_tc11_structure(&mut harness);
    run_tc11_camera_benchmarks(&mut harness);
    run_tc08_massive(&mut harness);
    run_tc12_chroma_and_tc20_anime(&mut harness);
    run_tc13_to_tc19_effects(&mut harness);
}
