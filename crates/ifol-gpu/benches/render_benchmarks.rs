use criterion::{criterion_group, criterion_main, Criterion};
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{RenderGraph, RenderNode, RenderTarget, ResourceRegistry, TextureHandle, RenderGraphExecutor, DrawCommand, MeshHandle, PipelineHandle, BindGroupHandle};
use std::borrow::Cow;

fn bench_clear_screen(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    
    // Tạo 1 Texture ảo trên VRAM để làm bia tập vẽ (Target)
    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // Đăng ký tài nguyên vào Registry
    let mut registry = ResourceRegistry::new();
    let tex_handle = TextureHandle(1);
    registry.textures.insert(tex_handle, target_view);

    // Dựng luồng RenderGraph (Chỉ Xóa Màn Hình - Không lệnh vẽ)
    let mut graph = RenderGraph::new();
    let target = RenderTarget {
        color_attachments: vec![tex_handle],
        depth_attachment: None,
    };
    graph.add_node(RenderNode::new("ClearPass", target));

    // Bắt đầu Benchmark!
    c.bench_function("bench_clear_screen", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); // Đồng bộ ép GPU chạy xong mới đếm thời gian
        })
    });
}

fn bench_empty_graph(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let registry = ResourceRegistry::new();
    let graph = RenderGraph::new(); // Đồ thị rỗng hoàn toàn, không có Node nào

    c.bench_function("bench_empty_graph", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        })
    });
}

fn bench_complex_graph(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    registry.textures.insert(TextureHandle(1), target_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(1), pipeline.clone());
    registry.pipelines.insert(PipelineHandle(2), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));

    let mut graph = RenderGraph::new();
    // 100 Nodes, each has 100 draw commands with alternating pipelines (100 * 100 = 10,000 commands)
    for i in 0..100 {
        let mut node = RenderNode::new(format!("Pass_{}", i), RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
        for j in 0..100 {
            let pipe = if j % 2 == 0 { PipelineHandle(1) } else { PipelineHandle(2) };
            node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: pipe, bind_groups: vec![], instance_count: 1 });
        }
        graph.add_node(node);
    }

    c.bench_function("bench_complex_graph_100_nodes", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        })
    });
}

fn bench_single_large_image(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    
    // 1. Tạo Target Texture
    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Target"),
        size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // 2. Load Shader
    let shader_src = std::fs::read_to_string("benches/assets/basic_texture.wgsl").unwrap();
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_src)),
    });

    // 3. Load Image (Khỏi cần parse nội dung thật trong benchmark nếu ta chỉ test tốc độ GPU. 
    //    Tuy nhiên, để WGPU thao tác VRAM thật, ta phải tạo một texture to)
    let ai_img_data = std::fs::read("benches/assets/ai_demo_large.png").unwrap();
    let img = image::load_from_memory(&ai_img_data).unwrap().to_rgba8();
    let dims = img.dimensions();
    
    let texture_size = wgpu::Extent3d {
        width: dims.0,
        height: dims.1,
        depth_or_array_layers: 1,
    };
    let src_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        label: Some("AI Large Image"),
        view_formats: &[],
    });
    engine.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &src_tex,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &img,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * dims.0),
            rows_per_image: Some(dims.1),
        },
        texture_size,
    );
    let src_view = src_tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = engine.device().create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // 4. BindGroupLayout & BindGroup
    let bgl = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        label: None,
    });
    let bg = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&src_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
        ],
        label: None,
    });

    // 5. Pipeline
    let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bgl)],
        immediate_size: 0,
    });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[], // Dùng Fullscreen Triangle thủ thuật hoặc Dummy Buffer
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });

    // 6. Đăng ký tài nguyên
    let mut registry = ResourceRegistry::new();
    registry.textures.insert(TextureHandle(1), target_view);
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.bind_groups.insert(BindGroupHandle(1), bg);
    
    // Mesh Dummy (Vẽ 3 đỉnh fullscreen triangle)
    registry.meshes.insert(MeshHandle(1), (
        engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }),
        None,
        3
    ));

    // 7. Graph
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("LargeImagePass", RenderTarget {
        color_attachments: vec![TextureHandle(1)],
        depth_attachment: None,
    });
    node.commands.push(DrawCommand::DrawMesh {
        mesh: MeshHandle(1),
        pipeline: PipelineHandle(1),
        bind_groups: vec![BindGroupHandle(1)],
        instance_count: 1,
    });
    graph.add_node(node);

    c.bench_function("bench_single_large_image", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        })
    });
}

fn bench_100k_sprites_cpu_stress(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();
    
    // Đăng ký Pipeline & Dummy Mesh
    // Trong thực tế, Pipeline này cần Shader đúng, nhưng để test CPU Compiler overhead thì dummy cũng được.
    registry.pipelines.insert(PipelineHandle(1), engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Dummy"),
        layout: Some(&engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 })),
        vertex: wgpu::VertexState {
            module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }")) }),
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) }),
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    }));
    
    registry.meshes.insert(MeshHandle(1), (
        engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }),
        None,
        3
    ));

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
    registry.textures.insert(TextureHandle(1), target_view);

    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("CPU_Stress_Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    
    // Nhồi 100,000 DrawCommand vào RenderGraph
    for _ in 0..100_000 {
        node.commands.push(DrawCommand::DrawMesh {
            mesh: MeshHandle(1),
            pipeline: PipelineHandle(1),
            bind_groups: vec![],
            instance_count: 1,
        });
    }
    graph.add_node(node);

    c.bench_function("bench_100k_sprites_cpu_stress", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        })
    });
}

fn bench_100k_sprites_gpu_instanced(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();
    
    registry.pipelines.insert(PipelineHandle(1), engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Dummy"),
        layout: Some(&engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 })),
        vertex: wgpu::VertexState {
            module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); }")) }),
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) }),
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    }));
    
    registry.meshes.insert(MeshHandle(1), (
        engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }),
        None,
        3
    ));

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());
    registry.textures.insert(TextureHandle(1), target_view);

    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("GPU_Instanced_Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    
    // Chỉ 1 DrawCommand, nhưng instance_count = 100,000
    node.commands.push(DrawCommand::DrawMesh {
        mesh: MeshHandle(1),
        pipeline: PipelineHandle(1),
        bind_groups: vec![],
        instance_count: 100_000,
    });
    graph.add_node(node);

    c.bench_function("bench_100k_sprites_gpu_instanced", |b| {
        b.iter(|| {
            let idx = executor.execute(&engine, &registry, &graph);
            let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
        })
    });
}

fn bench_z_buffer(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let target_view = target_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DepthTexture"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    registry.textures.insert(TextureHandle(1), target_view);
    registry.textures.insert(TextureHandle(2), depth_view);

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    let create_pipeline = |depth: bool| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            primitive: Default::default(),
            depth_stencil: if depth { Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }) } else { None },
            multisample: Default::default(), multiview_mask: None, cache: None,
        })
    };

    registry.pipelines.insert(PipelineHandle(1), create_pipeline(false)); // No Depth
    registry.pipelines.insert(PipelineHandle(2), create_pipeline(true)); // With Depth

    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));

    let mut graph_no_depth = RenderGraph::new();
    let mut node_no_depth = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node_no_depth.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 10000 });
    graph_no_depth.add_node(node_no_depth);

    let mut graph_with_depth = RenderGraph::new();
    let mut node_with_depth = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: Some(TextureHandle(2)) });
    node_with_depth.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 10000 });
    graph_with_depth.add_node(node_with_depth);

    c.bench_function("bench_z_buffer_disabled", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_no_depth); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
    c.bench_function("bench_z_buffer_enabled", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_with_depth); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
}

fn bench_alpha_blending(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    registry.textures.insert(TextureHandle(1), target_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    let create_pipeline = |blend: Option<wgpu::BlendState>| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
        })
    };

    registry.pipelines.insert(PipelineHandle(1), create_pipeline(Some(wgpu::BlendState::REPLACE)));
    registry.pipelines.insert(PipelineHandle(2), create_pipeline(Some(wgpu::BlendState::ALPHA_BLENDING)));
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));

    let mut graph_replace = RenderGraph::new();
    let mut node = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 10000 });
    graph_replace.add_node(node);

    let mut graph_alpha = RenderGraph::new();
    let mut node2 = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node2.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 10000 });
    graph_alpha.add_node(node2);

    c.bench_function("bench_alpha_blend_replace", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_replace); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
    c.bench_function("bench_alpha_blend_alpha", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_alpha); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
}

fn bench_pipeline_caching(c: &mut Criterion) {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let executor = RenderGraphExecutor::new();
    let mut registry = ResourceRegistry::new();

    let target_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("DummyTarget"), size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    registry.textures.insert(TextureHandle(1), target_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("@vertex fn vs() -> @builtin(position) vec4<f32> { return vec4(0.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4(1.0); }")) });
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });

    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    // We register the exact same pipeline object twice, simulating two different pipeline IDs (e.g. slight variant)
    registry.pipelines.insert(PipelineHandle(1), pipeline.clone());
    registry.pipelines.insert(PipelineHandle(2), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));

    let mut graph_sorted = RenderGraph::new();
    let mut node_sorted = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    for _ in 0..5000 { node_sorted.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 1 }); }
    for _ in 0..5000 { node_sorted.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 1 }); }
    graph_sorted.add_node(node_sorted);

    let mut graph_unsorted = RenderGraph::new();
    let mut node_unsorted = RenderNode::new("Pass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    for _ in 0..5000 {
        node_unsorted.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 1 });
        node_unsorted.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 1 });
    }
    graph_unsorted.add_node(node_unsorted);

    c.bench_function("bench_pipeline_state_sorted", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_sorted); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
    c.bench_function("bench_pipeline_state_unsorted", |b| { b.iter(|| { let idx = executor.execute(&engine, &registry, &graph_unsorted); let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None }); }) });
}

criterion_group!(benches, bench_clear_screen, bench_empty_graph, bench_complex_graph, bench_single_large_image, bench_100k_sprites_cpu_stress, bench_100k_sprites_gpu_instanced, bench_z_buffer, bench_alpha_blending, bench_pipeline_caching);
criterion_main!(benches);
