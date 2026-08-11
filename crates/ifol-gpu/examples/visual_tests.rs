use std::borrow::Cow;
use ifol_gpu::api::GpuEngineBuilder;
use ifol_gpu::render::{RenderGraph, RenderNode, RenderTarget, ResourceRegistry, TextureHandle, RenderGraphExecutor, DrawCommand, MeshHandle, PipelineHandle, BindGroupHandle};

fn main() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).expect("Failed to build engine");
    let executor = RenderGraphExecutor::new();
    
    // Create textures
    let width = 800;
    let height = 600;
    
    let create_target = || {
        let tex = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("Target"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[],
        });
        (tex.create_view(&wgpu::TextureViewDescriptor::default()), tex)
    };

    let (z_target_view, z_target_tex) = create_target();
    let (alpha_target_view, alpha_target_tex) = create_target();

    let depth_tex = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("Depth"), size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
    });
    let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    let mut registry = ResourceRegistry::new();
    registry.textures.insert(TextureHandle(1), z_target_view);
    registry.textures.insert(TextureHandle(2), depth_view);
    registry.textures.insert(TextureHandle(3), alpha_target_view);

    // Shaders
    // A shader that draws colored triangles based on vertex index (0..3 is one triangle, etc).
    // We can use built-in vertex_index to position them without a vertex buffer.
    let shader_src = "
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
        };

        @vertex
        fn vs_main(
            @builtin(vertex_index) in_vertex_index: u32,
            @builtin(instance_index) in_instance_index: u32,
        ) -> VertexOutput {
            var out: VertexOutput;
            // Simple hardcoded triangles
            let x = f32(i32(in_vertex_index) - 1) * 0.5;
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
            
            // Offset based on instance
            let offset_x = f32(in_instance_index) * 0.2 - 0.2;
            let offset_y = f32(in_instance_index) * 0.2 - 0.2;
            // Depth based on instance: higher instance = further from camera (higher Z in wgpu)
            // This ensures that Red (instance 0, drawn first) has Z=0.4 (closest),
            // Green (instance 1) has Z=0.6, Blue (instance 2) has Z=0.8 (furthest).
            // This is the true test of Z-buffer: Red should be ON TOP even though drawn first.
            let z = 0.4 + f32(in_instance_index) * 0.2;

            out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
            
            // Colors for instances
            if (in_instance_index == 0u) {
                out.color = vec4<f32>(1.0, 0.0, 0.0, 0.5); // Red half transparent
            } else if (in_instance_index == 1u) {
                out.color = vec4<f32>(0.0, 1.0, 0.0, 0.5); // Green half transparent
            } else {
                out.color = vec4<f32>(0.0, 0.0, 1.0, 0.5); // Blue half transparent
            }
            return out;
        }

        @fragment
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
            return in.color;
        }
    ";
    
    let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("VisualShader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
    });
    
    let layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None, bind_group_layouts: &[], immediate_size: 0,
    });

    let create_pipe = |depth: bool, blend: Option<wgpu::BlendState>| {
        engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { 
                module: &shader, entry_point: Some("fs_main"), 
                targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend, write_mask: wgpu::ColorWrites::ALL })], 
                compilation_options: Default::default() 
            }),
            primitive: Default::default(),
            depth_stencil: if depth { Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }) } else { None },
            multisample: Default::default(), multiview_mask: None, cache: None,
        })
    };

    // Pipeline 1: Depth Enabled, Solid (Blend Replace)
    registry.pipelines.insert(PipelineHandle(1), create_pipe(true, Some(wgpu::BlendState::REPLACE)));
    // Pipeline 2: Depth Disabled, Alpha Blending
    registry.pipelines.insert(PipelineHandle(2), create_pipe(false, Some(wgpu::BlendState::ALPHA_BLENDING)));
    
    // Dummy mesh (3 vertices)
    let dummy_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor { size: 4, usage: wgpu::BufferUsages::VERTEX, label: None, mapped_at_creation: false });
    registry.meshes.insert(MeshHandle(1), (dummy_buffer, None, 3));

    // --- Z-Buffer Test ---
    let mut graph_z = RenderGraph::new();
    let mut node_z = RenderNode::new("ZBufferTest", RenderTarget { color_attachments: vec![TextureHandle(1)], depth_attachment: Some(TextureHandle(2)) });
    node_z.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(1), bind_groups: vec![], instance_count: 3 });
    graph_z.add_node(node_z);
    
    // --- Alpha Test ---
    let mut graph_alpha = RenderGraph::new();
    let mut node_alpha = RenderNode::new("AlphaTest", RenderTarget { color_attachments: vec![TextureHandle(3)], depth_attachment: None });
    node_alpha.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 3 });
    graph_alpha.add_node(node_alpha);

    // --- 10K Objects Test ---
    let shader_10k_src = "
        struct VertexOutput {
            @builtin(position) clip_position: vec4<f32>,
            @location(0) color: vec4<f32>,
        };

        @vertex
        fn vs_main(
            @builtin(vertex_index) in_vertex_index: u32,
            @builtin(instance_index) in_instance_index: u32,
        ) -> VertexOutput {
            var out: VertexOutput;
            // Tiny triangles
            let x = f32(i32(in_vertex_index) - 1) * 0.015;
            let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.015;
            
            // 100x100 grid layout
            let row = f32(in_instance_index / 100u);
            let col = f32(in_instance_index % 100u);
            let offset_x = (col / 100.0) * 2.0 - 1.0 + 0.01;
            let offset_y = (row / 100.0) * 2.0 - 1.0 + 0.01;
            let z = 0.5;

            out.clip_position = vec4<f32>(x + offset_x, y + offset_y, z, 1.0);
            
            // Gradient color based on position
            out.color = vec4<f32>(col / 100.0, row / 100.0, 1.0 - (col+row)/200.0, 1.0);
            return out;
        }

        @fragment
        fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
            return in.color;
        }
    ";
    let shader_10k = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("VisualShader10k"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_10k_src)),
    });
    
    let pipe_10k = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None, layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader_10k, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState { 
            module: &shader_10k, entry_point: Some("fs_main"), 
            targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8UnormSrgb, blend: Some(wgpu::BlendState::REPLACE), write_mask: wgpu::ColorWrites::ALL })], 
            compilation_options: Default::default() 
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(), multiview_mask: None, cache: None,
    });
    registry.pipelines.insert(PipelineHandle(3), pipe_10k);
    
    let (target_10k_view, target_10k_tex) = create_target();
    registry.textures.insert(TextureHandle(4), target_10k_view);

    let mut graph_10k = RenderGraph::new();
    let mut node_10k = RenderNode::new("10kTest", RenderTarget { color_attachments: vec![TextureHandle(4)], depth_attachment: None });
    node_10k.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(3), bind_groups: vec![], instance_count: 10000 });
    graph_10k.add_node(node_10k);

    // --- State Tracking Accuracy Test (Interleaved Pipelines) ---
    // Test xem Engine có xử lý chính xác khi chuyển đổi Pipeline liên tục không
    let (target_interleaved_view, target_interleaved_tex) = create_target();
    registry.textures.insert(TextureHandle(5), target_interleaved_view);
    
    let mut graph_interleaved = RenderGraph::new();
    let mut node_interleaved = RenderNode::new("InterleavedTest", RenderTarget { color_attachments: vec![TextureHandle(5)], depth_attachment: None });
    // Vẽ đan xen: Pipe 2 (Alpha) -> Pipe 3 (10k/Replace) -> Pipe 2 (Alpha) -> Pipe 3 (10k/Replace)
    // Cả hai Pipeline đều Không có Depth attachment
    node_interleaved.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 1 });
    node_interleaved.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(3), bind_groups: vec![], instance_count: 1 });
    node_interleaved.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(2), bind_groups: vec![], instance_count: 1 });
    node_interleaved.commands.push(DrawCommand::DrawMesh { mesh: MeshHandle(1), pipeline: PipelineHandle(3), bind_groups: vec![], instance_count: 1 });
    graph_interleaved.add_node(node_interleaved);

    // Execute
    let _ = executor.execute(&engine, &registry, &graph_z);
    let _ = executor.execute(&engine, &registry, &graph_alpha);
    let _ = executor.execute(&engine, &registry, &graph_10k);
    let idx_last = executor.execute(&engine, &registry, &graph_interleaved);
    
    // Đợi GPU chạy xong lệnh cuối cùng
    let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(idx_last), timeout: None });

    // Helper to readback sử dụng API mới
    fn save_texture(engine: &ifol_gpu::api::GpuEngine, texture: &wgpu::Texture, filename: &str) {
        let path = std::path::Path::new("examples/outputs").join(filename);
        engine.save_texture_to_file(texture, &path).expect("Lỗi lưu ảnh");
    }

    save_texture(&engine, &z_target_tex, "z_buffer_test.png");
    save_texture(&engine, &alpha_target_tex, "alpha_test.png");
    save_texture(&engine, &target_10k_tex, "10k_test.png");
    save_texture(&engine, &target_interleaved_tex, "interleaved_test.png");
}
