use std::borrow::Cow;
use std::time::Instant;
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{RenderGraph, RenderNode, RenderTarget, ResourceRegistry, TextureHandle, RenderGraphExecutor, DrawCommand, MeshHandle, PipelineHandle};

// Helper: Khởi tạo Texture làm bia vẽ (Target)
fn create_target(engine: &ifol_gpu::api::GpuEngine) -> (wgpu::TextureView, wgpu::Texture) {
    let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Target"), size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
    });
    (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
}

// Helper: Lưu ảnh
fn save_texture(engine: &ifol_gpu::api::GpuEngine, texture: &wgpu::Texture, filename: &str) {
    let path = std::path::Path::new("examples/outputs").join(filename);
    engine.save_texture_to_file(texture, &path).expect("Lỗi lưu ảnh");
}

fn test_01_clear_color(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    let mut graph = RenderGraph::new();
    graph.add_node(RenderNode::new("ClearPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None }));
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 01 (Clear Color) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_01_clear_color.png");
}

fn test_02_z_buffer(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"), size: wgpu::Extent3d { width: 800, height: 600, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    registry.textures.insert(TextureHandle(2), depth_tex.create_view(&wgpu::TextureViewDescriptor::default()));

    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            @vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32, @builtin(instance_index) in_instance_index: u32) -> VertexOutput {
                var out: VertexOutput;
                let x = f32(i32(in_vertex_index) - 1) * 0.5;
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                let offset_x = f32(in_instance_index) * 0.2 - 0.2;
                let offset_y = f32(in_instance_index) * 0.2 - 0.2;
                let z = 0.8 - f32(in_instance_index) * 0.3;
                out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
                if (in_instance_index == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); } 
                else if (in_instance_index == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 1.0); } 
                else { out.color = vec4<f32>(0.0, 0.0, 1.0, 1.0); }
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        "))
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }), multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));
    
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("ZBufferPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: Some(TextureHandle(2)) });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 3 });
    graph.add_node(node);
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 02 (Z-Buffer) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_02_z_buffer.png");
}

fn test_03_alpha_blend(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            @vertex fn vs_main(@builtin(vertex_index) in_vertex_index: u32, @builtin(instance_index) in_instance_index: u32) -> VertexOutput {
                var out: VertexOutput;
                let x = f32(i32(in_vertex_index) - 1) * 0.5;
                let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                let offset_x = f32(in_instance_index) * 0.2 - 0.2;
                let offset_y = f32(in_instance_index) * 0.2 - 0.2;
                out.clip_position = vec4<f32>(x + offset_x, y + offset_y, 0.5, 1.0);
                if (in_instance_index == 0u) { out.color = vec4<f32>(1.0, 0.0, 0.0, 0.5); } 
                else if (in_instance_index == 1u) { out.color = vec4<f32>(0.0, 1.0, 0.0, 0.5); } 
                else { out.color = vec4<f32>(0.0, 0.0, 1.0, 0.5); } 
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        "))
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));
    
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("AlphaPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 3 });
    graph.add_node(node);
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 03 (Alpha Blend) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_03_alpha_blend.png");
}

fn test_04_interleaved(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    use wgpu::util::DeviceExt;
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    // Shader vẽ một Quad (tứ giác) 6 đỉnh với Uniform Offset
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            
            @group(0) @binding(0)
            var<uniform> offset: vec2<f32>;

            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-0.2, -0.2), vec2<f32>(0.2, -0.2), vec2<f32>(-0.2, 0.2),
                    vec2<f32>(-0.2, 0.2), vec2<f32>(0.2, -0.2), vec2<f32>(0.2, 0.2)
                );
                var out: VertexOutput;
                out.clip_position = vec4<f32>(pos[id].x + offset.x, pos[id].y + offset.y, 0.5, 1.0);
                out.color = vec4<f32>(1.0, 1.0, 0.0, 0.5); // Vàng trong suốt 50%
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        "))
    });

    let bind_group_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None,
            }, count: None,
        }], label: None,
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bind_group_layout)], immediate_size: 0 });
    
    let pipe_alpha = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    let pipe_solid = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });

    registry.pipelines.insert(PipelineHandle(1), pipe_alpha);
    registry.pipelines.insert(PipelineHandle(2), pipe_solid);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 6));

    // Tạo 4 BindGroup với 4 offset khác nhau
    for i in 0..4 {
        let offset = [f32::from(i as i16) * 0.5 - 0.75, 0.0_f32];
        let buffer = engine.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None, contents: bytemuck::cast_slice(&offset), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bg = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }], label: None,
        });
        registry.bind_groups.insert(ifol_gpu::render::BindGroupHandle(i + 1), bg);
    }
    
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("InterleavedPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    // Vẽ đan xen: 1 Alpha, 1 Solid, 1 Alpha, 1 Solid
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![ifol_gpu::render::BindGroupHandle(1)], instance_count: 1 });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![ifol_gpu::render::BindGroupHandle(2)], instance_count: 1 });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![ifol_gpu::render::BindGroupHandle(3)], instance_count: 1 });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![ifol_gpu::render::BindGroupHandle(4)], instance_count: 1 });
    graph.add_node(node);
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 04 (Interleaved) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_04_interleaved.png");
}

fn test_05_garbage_collection(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(id) - 1) * 0.5; let y = f32(i32(id & 1u) * 2 - 1) * 0.5;
                return vec4(x, y, 0.5, 1.0);
            }
            @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4(1.0, 0.0, 0.0, 1.0); }
        "))
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3)); 
    registry.meshes.insert(MeshHandle(2), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3)); 
    
    registry.remove_mesh(&MeshHandle(2));

    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("GCPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(2), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 1 }); 
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 1 }); 
    graph.add_node(node);
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 05 (Garbage Collection) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_05_garbage_collection.png");
}

fn test_07_complex_frame(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    // Shader vẽ một Quad (tứ giác) ngẫu nhiên thay vì tam giác 3 đỉnh
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            struct VertexOutput { @builtin(position) clip_position: vec4<f32>, @location(0) color: vec4<f32>, };
            
            fn rand(seed: u32) -> f32 {
                var s = seed;
                s = (s ^ 61u) ^ (s >> 16u);
                s = s * 9u;
                s = s ^ (s >> 4u);
                s = s * 668265261u;
                s = s ^ (s >> 15u);
                return f32(s) / 4294967296.0;
            }

            @vertex fn vs_main(@builtin(vertex_index) id: u32, @builtin(instance_index) inst: u32) -> VertexOutput {
                // Tọa độ 6 đỉnh tạo thành hình tứ giác (Quad)
                var pos = array<vec2<f32>, 6>(
                    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(-1.0, 1.0),
                    vec2<f32>(-1.0, 1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0)
                );

                var out: VertexOutput;
                // Kích thước hạt bụi (0.005 đến 0.05)
                let size = 0.005 + rand(inst * 13u) * 0.045;
                
                // Vị trí ngẫu nhiên (-1.0 đến 1.0)
                let pos_x = rand(inst * 19u) * 2.0 - 1.0;
                let pos_y = rand(inst * 23u) * 2.0 - 1.0;
                
                out.clip_position = vec4<f32>(pos[id].x * size + pos_x, pos[id].y * size + pos_y, 0.5, 1.0);
                
                let r = rand(inst * 31u);
                let g = rand(inst * 37u);
                let b = rand(inst * 41u);
                let a = 0.3 + rand(inst * 43u) * 0.7; // Độ trong suốt 0.3 đến 1.0
                out.color = vec4<f32>(r, g, b, a);
                return out;
            }
            @fragment fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> { return in.color; }
        "))
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    // Dummy Mesh với count = 6 để GPU Loop qua 6 đỉnh tứ giác
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 6));
    
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("ComplexFramePass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 50_000 });
    graph.add_node(node);
    
    let start = Instant::now();
    let idx = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx), timeout: None });
    let duration = start.elapsed();
    
    println!("Test 07 (Complex Particles Quad) Render Time: {:?}", duration);
    save_texture(engine, &tex, "test_07_complex_frame.png");
}

fn test_08_multi_graph_cache(engine: &ifol_gpu::api::GpuEngine, executor: &RenderGraphExecutor) {
    let mut registry = ResourceRegistry::new();
    let (view, tex) = create_target(engine);
    registry.textures.insert(TextureHandle(1), view);
    
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None, source: wgpu::ShaderSource::Wgsl(Cow::Borrowed("
            @vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4<f32> {
                let x = f32(i32(id) - 1) * 0.5; let y = f32(i32(id & 1u) * 2 - 1) * 0.5;
                return vec4(x, y, 0.5, 1.0);
            }
            @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4(0.0, 1.0, 0.0, 1.0); }
        "))
    });

    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
    let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
        primitive: Default::default(), depth_stencil: None, multisample: Default::default(), multiview_mask: None, cache: None,
    });
    
    registry.pipelines.insert(PipelineHandle(1), pipeline);
    registry.meshes.insert(MeshHandle(1), (engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false }), None, 3));
    
    let mut graph = RenderGraph::new();
    let mut node = RenderNode::new("MultiGraphPass", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: None });
    for _ in 0..10_000 {
        node.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 1 }); 
    }
    graph.add_node(node);
    
    // Pass 1
    let start_1 = Instant::now();
    let idx_1 = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_1), timeout: None });
    let duration_1 = start_1.elapsed();
    
    // Pass 2 (Ngay lập tức đẩy lại graph đó lần nữa)
    let start_2 = Instant::now();
    let idx_2 = executor.execute(engine, &registry, &graph);
    engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_2), timeout: None });
    let duration_2 = start_2.elapsed();
    
    println!("Test 08 (Multi Graph Cache) - Run 1: {:?}, Run 2: {:?}", duration_1, duration_2);
    save_texture(engine, &tex, "test_08_multi_graph.png");
}

fn main() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    let executor = RenderGraphExecutor::new();
    
    std::fs::create_dir_all("examples/outputs").unwrap();
    
    println!("Running Test 01..."); test_01_clear_color(&engine, &executor);
    println!("Running Test 02..."); test_02_z_buffer(&engine, &executor);
    println!("Running Test 03..."); test_03_alpha_blend(&engine, &executor);
    println!("Running Test 04..."); test_04_interleaved(&engine, &executor);
    println!("Running Test 05..."); test_05_garbage_collection(&engine, &executor);
    println!("Running Test 07..."); test_07_complex_frame(&engine, &executor);
    println!("Running Test 08..."); test_08_multi_graph_cache(&engine, &executor);
    
    println!("Tất cả các bài test đã sinh ảnh thành công trong thư mục examples/outputs/");
}
