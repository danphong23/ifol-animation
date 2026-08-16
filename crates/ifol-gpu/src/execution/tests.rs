    use super::{RenderGraphExecutor, RenderGraphValidationError};
    use crate::backend::GpuEngineBuilder;
    use crate::graph::{ComputeCommand, CopyCommand, DrawAction, DrawCommand, GraphResource, RenderGraph, RenderNodePool, RenderTarget, ResourceAccess};
    use crate::resources::{BindGroupHandle, BindGroupResourceDescriptor, BufferHandle, BufferResourceDescriptor, ComputePipelineHandle, PipelineHandle, PipelineLayoutResourceDescriptor, ResourceRegistry, TextureHandle, TextureResourceDescriptor};

    #[test]
    fn copy_only_graph_executes_without_render_target() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let source = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("copy_source"), size: 4,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let destination = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("copy_destination"), size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        engine.queue().write_buffer(&source, 0, &[7, 8, 9, 10]);

        let mut registry = ResourceRegistry::new();
        registry.insert_buffer_with_descriptor(BufferHandle(1), source, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), destination, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ }).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission.clone()), timeout: None });
        let destination = registry.buffer(&BufferHandle(2)).unwrap();
        let slice = destination.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        receiver.recv().unwrap().unwrap();
        assert_eq!(&*slice.get_mapped_range().unwrap(), &[7, 8, 9, 10]);
    }

    #[test]
    fn texture_copy_graph_executes_and_preserves_pixels() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            mip_level_count: 1,
            sample_count: 1,
        };
        let create_texture = |label| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        });
        let source = create_texture("texture_copy_source");
        let destination = create_texture("texture_copy_destination");
        let pixels = [
            255, 0, 0, 255, 0, 255, 0, 255,
            0, 0, 255, 255, 255, 255, 255, 255,
        ];
        engine.queue().write_texture(
            source.as_image_copy(),
            &pixels,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(8), rows_per_image: Some(2) },
            wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
        );

        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), source, descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(2), destination, descriptor, 1024).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [2, 2, 1])]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let readback = engine.read_texture_to_raw_with_format_checked(registry.owned_texture(&TextureHandle(2)).unwrap(), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        assert_eq!((readback.width, readback.height), (2, 2));
        assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(readback.bytes, pixels);
    }

    #[test]
    fn texture_copy_validation_rejects_missing_ownership_and_out_of_bounds_extent() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor {
            width: 4, height: 4, depth_or_array_layers: 1,
            format: wgpu::TextureFormat::Rgba8Unorm, usage,
            mip_level_count: 1, sample_count: 1,
        };
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("validation_texture"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), texture, descriptor, 1024).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [1, 1, 1])]);
        assert_eq!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::MissingTexture(TextureHandle(2))));

        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("validation_texture_destination"),
            size: wgpu::Extent3d { width: 4, height: 4, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        registry.insert_owned_texture(TextureHandle(2), texture, descriptor, 1024).unwrap();
        graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [5, 3, 1])]);
        assert!(matches!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::InvalidTextureCopyRange { .. })));
    }

    #[test]
    fn target_graph_with_interleaved_copy_and_draw_uses_ordered_segments() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ordered_segments_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> { var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0)); return vec4<f32>(p[i], 0.0, 1.0); } @fragment fn fs() -> @location(0) vec4<f32> { return vec4<f32>(1.0, 0.0, 0.0, 1.0); }",
            )),
        });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ordered_segments_pipeline"),
            layout: None,
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })], compilation_options: Default::default() }),
            multiview_mask: None,
            cache: None,
        });
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST;
        let descriptor = TextureResourceDescriptor { width: 2, height: 2, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 };
        let make_texture = |label| engine.device().create_texture(&wgpu::TextureDescriptor { label: Some(label), size: wgpu::Extent3d { width: 2, height: 2, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[] });
        let mut registry = ResourceRegistry::new();
        registry.insert_owned_texture(TextureHandle(1), make_texture("ordered_source"), descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(2), make_texture("ordered_copy_destination"), descriptor, 1024).unwrap();
        registry.insert_owned_texture(TextureHandle(3), make_texture("ordered_target"), descriptor, 1024).unwrap();
        registry.insert_pipeline_with_layout_descriptor(
            PipelineHandle(1),
            pipeline,
            PipelineLayoutResourceDescriptor { bind_group_layout_signatures: Vec::new() },
        );
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 2, height: 2 });
        graph.add_copy_batch(&mut pool, vec![CopyCommand::texture_to_texture(TextureHandle(1), TextureHandle(2), [2, 2, 1])]);
        graph.add_batch(&mut pool, vec![DrawCommand::new(PipelineHandle(1), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })]);
        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let readback = engine.read_texture_to_raw_with_format_checked(registry.owned_texture(&TextureHandle(3)).unwrap(), wgpu::TextureFormat::Rgba8Unorm).unwrap();
        assert_eq!(readback.format, wgpu::TextureFormat::Rgba8Unorm);
        assert_eq!(&readback.bytes[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn compute_only_graph_executes_storage_update_without_render_target() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compute_test"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute_test_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("compute_test_pipeline_layout"), bind_group_layouts: &[Some(&layout)], immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_test_pipeline"), layout: Some(&pipeline_layout), module: &shader,
            entry_point: Some("main"), compilation_options: Default::default(), cache: None,
        });
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute_test_buffer"), size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("compute_test_staging"), size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        engine.queue().write_buffer(&buffer, 0, bytemuck::bytes_of(&0u32));
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute_test_bind_group"), layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer_with_descriptor(BufferHandle(1), buffer, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), staging, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ }).unwrap();
        registry.insert_compute_pipeline_with_layout_descriptor(ComputePipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(1)] });
        registry.insert_bind_group_with_descriptor(BindGroupHandle(1), bind_group, BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1 }).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![])]);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission.clone()), timeout: None });
        let staging = registry.buffer(&BufferHandle(2)).unwrap();
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        receiver.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().unwrap();
        assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 1);
    }

    #[test]
    fn flattened_execution_preserves_root_before_nested_compute_order() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nested_order_compute"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<storage, read_write> data: array<u32>; @compute @workgroup_size(1) fn main() { data[0] = data[0] + 1u; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nested_order_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nested_order_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("nested_order_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let make_buffer = |label, usage| engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: 4,
            usage,
            mapped_at_creation: false,
        });
        let source = make_buffer("nested_order_source", wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        let shared = make_buffer("nested_order_shared", wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST);
        let staging = make_buffer("nested_order_staging", wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        engine.queue().write_buffer(&source, 0, bytemuck::bytes_of(&7u32));
        engine.queue().write_buffer(&shared, 0, bytemuck::bytes_of(&0u32));
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nested_order_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: shared.as_entire_binding() }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer_with_descriptor(BufferHandle(1), source, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), shared, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(3), staging, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ }).unwrap();
        registry.insert_compute_pipeline_with_layout_descriptor(ComputePipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(1)] });
        registry.insert_bind_group_with_descriptor(BindGroupHandle(1), bind_group, BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 1 }).unwrap();

        let mut pool = RenderNodePool::new();
        let mut child = RenderGraph::new(RenderTarget::Screen);
        let child_compute = child.add_compute_batch(&mut pool, vec![
            ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![]),
        ]);
        child.declare_resource_usage(child_compute, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::ReadWrite);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        root.add_subgraph(&mut pool, "nested-order", child, vec![]);
        root.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(2), BufferHandle(3), 4)]);

        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut pool, &root).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
        let staging = registry.buffer(&BufferHandle(3)).unwrap();
        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| { let _ = sender.send(result); });
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        receiver.recv().unwrap().unwrap();
        let bytes = slice.get_mapped_range().unwrap();
        assert_eq!(u32::from_ne_bytes(bytes[0..4].try_into().unwrap()), 8);
    }

    #[test]
    fn validation_checks_descriptor_aware_dynamic_offsets() {
        let engine = pollster::block_on(GpuEngineBuilder::new().with_required_limits(wgpu::Limits::default()).build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("dynamic_offset_validation_shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                "@group(0) @binding(0) var<uniform> value: u32; @compute @workgroup_size(1) fn main() { _ = value; }",
            )),
        });
        let layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("dynamic_offset_validation_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("dynamic_offset_validation_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("dynamic_offset_validation_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let alignment = engine.capabilities().min_uniform_buffer_offset_alignment;
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("dynamic_offset_validation_buffer"),
            size: alignment as u64 * 2,
            usage: wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_validation_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
        let mismatched_bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("dynamic_offset_mismatched_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(4),
                }),
            }],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_compute_pipeline_with_layout_descriptor(
            ComputePipelineHandle(1),
            pipeline,
            crate::resources::PipelineLayoutResourceDescriptor {
                bind_group_layout_signatures: vec![Some(7)],
            },
        );
        registry
            .insert_bind_group_with_descriptor(
                BindGroupHandle(1),
                bind_group,
                crate::resources::BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment,
                    layout_signature: 7,
                },
            )
            .unwrap();
        registry
            .insert_bind_group_with_descriptor(
                BindGroupHandle(2),
                mismatched_bind_group,
                crate::resources::BindGroupResourceDescriptor {
                    dynamic_offset_count: 1,
                    dynamic_offset_alignment: alignment,
                    layout_signature: 8,
                },
            )
            .unwrap();
        let mut pool = RenderNodePool::new();
        let mut valid = RenderGraph::new(RenderTarget::Screen);
        valid.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![alignment])]);
        assert_eq!(RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &valid), Ok(()));
        let mut invalid = RenderGraph::new(RenderTarget::Screen);
        invalid.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![1])]);
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &invalid),
            Err(RenderGraphValidationError::InvalidDynamicOffsetAlignment {
                handle: BindGroupHandle(1), offset: 1, alignment,
            })
        );
        let mut mismatched = RenderGraph::new(RenderTarget::Screen);
        mismatched.add_compute_batch(&mut pool, vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(2), vec![alignment])]);
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &mismatched),
            Err(RenderGraphValidationError::ComputePipelineLayoutMismatch {
                pipeline: ComputePipelineHandle(1), slot: 0, expected: Some(7), actual: Some(8),
            })
        );
    }

    use crate::resources::handle::*;

    #[test]
    fn validation_rejects_missing_texture_usage_for_depth() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(99),
            engine.device().create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: 100, height: 100, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Depth24Plus, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 100, height: 100, depth_or_array_layers: 1, mip_level_count: 1, sample_count: 1, format: wgpu::TextureFormat::Depth24Plus, usage: wgpu::TextureUsages::TEXTURE_BINDING },
            100
        ).unwrap();
        let pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.depth_stencil = Some(TextureHandle(99));
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::MissingTextureUsage { handle: TextureHandle(99), required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(), actual_usage: wgpu::TextureUsages::TEXTURE_BINDING.bits() })
        );
    }

    #[test]
    fn validation_rejects_depth_sample_count_mismatch() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(1),
            engine.device().create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: 100, height: 100, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 4, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] }).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 100, height: 100, depth_or_array_layers: 1, mip_level_count: 1, sample_count: 4, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT },
            100
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(2),
            engine.device().create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: 100, height: 100, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] }).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 100, height: 100, depth_or_array_layers: 1, mip_level_count: 1, sample_count: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::RENDER_ATTACHMENT },
            100
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(99),
            engine.device().create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: 100, height: 100, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Depth24Plus, usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[] }).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 100, height: 100, depth_or_array_layers: 1, mip_level_count: 1, sample_count: 1, format: wgpu::TextureFormat::Depth24Plus, usage: wgpu::TextureUsages::RENDER_ATTACHMENT },
            100
        ).unwrap();
        let pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::OffscreenMsaa { color: TextureHandle(1), resolve: TextureHandle(2), width: 100, height: 100 });
        graph.depth_stencil = Some(TextureHandle(99));
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::DepthSampleCountMismatch { handle: TextureHandle(99), expected: 4, actual: 1 })
        );
    }

    #[test]
    fn validation_rejects_render_pipeline_layout_mismatch() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        
        let mut registry = ResourceRegistry::new();
        registry.insert_pipeline_with_layout_descriptor(PipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(10)] });
        
        let bg_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[] });
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bg_layout, entries: &[] });
        registry.insert_bind_group_with_descriptor(BindGroupHandle(1), bind_group, BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 11 }).unwrap();
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let id = pool.alloc_batch(vec![DrawCommand::new(PipelineHandle(1), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 }).with_bind_group(0, BindGroupHandle(1), vec![])]);
        graph.add_node_id(id);
        
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::PipelineLayoutMismatch { pipeline: PipelineHandle(1), slot: 0, expected: Some(10), actual: Some(11) })
        );
    }

    #[test]
    fn validation_rejects_compute_pipeline_layout_mismatch() {
        let mut builder = GpuEngineBuilder::new();
        builder = builder.with_required_limits(wgpu::Limits { max_compute_invocations_per_workgroup: 256, max_compute_workgroup_size_x: 256, max_compute_workgroup_size_y: 256, max_compute_workgroup_size_z: 64, ..Default::default() });
        let engine = pollster::block_on(builder.build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@compute @workgroup_size(1) fn main() {}")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: None, layout: Some(&pipeline_layout), module: &shader, entry_point: Some("main"), compilation_options: Default::default(), cache: None });
        
        let mut registry = ResourceRegistry::new();
        registry.insert_compute_pipeline_with_layout_descriptor(ComputePipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![Some(10)] });
        
        let bg_layout = engine.device().create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[] });
        let bind_group = engine.device().create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bg_layout, entries: &[] });
        registry.insert_bind_group_with_descriptor(BindGroupHandle(1), bind_group, BindGroupResourceDescriptor { dynamic_offset_count: 0, dynamic_offset_alignment: 0, layout_signature: 11 }).unwrap();
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let id = pool.alloc_compute_batch(vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1]).with_bind_group(0, BindGroupHandle(1), vec![])]);
        graph.add_node_id(id);
        
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::ComputePipelineLayoutMismatch { pipeline: ComputePipelineHandle(1), slot: 0, expected: Some(10), actual: Some(11) })
        );
    }

    #[test]
    fn validation_rejects_missing_mesh_for_indexed_indirect() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        
        let mut registry = ResourceRegistry::new();
        registry.insert_pipeline_with_layout_descriptor(PipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![] });
        
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: 16, usage: wgpu::BufferUsages::INDIRECT, mapped_at_creation: false });
        registry.insert_buffer_with_descriptor(BufferHandle(1), buffer, BufferResourceDescriptor { size: 16, usage: wgpu::BufferUsages::INDIRECT }).unwrap();
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let id = pool.alloc_batch(vec![DrawCommand::new(PipelineHandle(1), DrawAction::IndexedIndirect { mesh: MeshHandle(99), buffer: BufferHandle(1), offset: 0 })]);
        graph.add_node_id(id);
        
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::MissingMesh(MeshHandle(99)))
        );
    }

    #[test]
    fn validation_rejects_missing_buffer_for_indirect() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        
        let mut registry = ResourceRegistry::new();
        registry.insert_pipeline_with_layout_descriptor(PipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![] });
        
        let index_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: 2, usage: wgpu::BufferUsages::INDEX, mapped_at_creation: false }); let mesh = (engine.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: 4, usage: wgpu::BufferUsages::VERTEX, mapped_at_creation: false }), Some((index_buffer, wgpu::IndexFormat::Uint16)), 1);
        registry.insert_mesh_with_descriptor(MeshHandle(1), mesh, crate::resources::MeshResourceDescriptor { vertex_count: 1, index_buffer_size: Some(2), index_format: Some(wgpu::IndexFormat::Uint16), vertex_buffer_size: 4 }).unwrap();
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let id = pool.alloc_batch(vec![DrawCommand::new(PipelineHandle(1), DrawAction::IndexedIndirect { mesh: MeshHandle(1), buffer: BufferHandle(99), offset: 0 })]);
        graph.add_node_id(id);
        
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::MissingIndirectBuffer(BufferHandle(99)))
        );
    }

    #[test]
    fn validation_rejects_missing_mesh_for_indexed() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        
        let mut registry = ResourceRegistry::new();
        registry.insert_pipeline_with_layout_descriptor(PipelineHandle(1), pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![] });
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let id = pool.alloc_batch(vec![DrawCommand::new(PipelineHandle(1), DrawAction::Indexed { mesh: MeshHandle(99), index_range: 0..3, instance_range: 0..1 })]);
        graph.add_node_id(id);
        
        assert_eq!(
            RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
            Err(RenderGraphValidationError::MissingMesh(MeshHandle(99)))
        );
    }

    #[test]
    fn execution_empty_graph_does_not_crash() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let registry = ResourceRegistry::new();
        let mut pool = RenderNodePool::new();
        let graph = RenderGraph::new(RenderTarget::Screen);
        
        let executor = RenderGraphExecutor::new();
        assert_eq!(executor.validate_with_device(&engine, &registry, &pool, &graph), Ok(()));
        let report = executor.execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None).unwrap();
        assert_eq!(report.flattened_nodes, 0);
        assert_eq!(report.draw_commands, 0);
    }

    #[test]
    fn execution_deeply_nested_subgraphs() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let registry = ResourceRegistry::new();
        let mut pool = RenderNodePool::new();
        
        let child3 = RenderGraph::new(RenderTarget::Screen);
        let id3 = pool.alloc_subgraph("child3".to_string(), child3, vec![]);
        
        let mut child2 = RenderGraph::new(RenderTarget::Screen);
        child2.add_node_id(id3);
        let id2 = pool.alloc_subgraph("child2".to_string(), child2, vec![]);
        
        let mut child1 = RenderGraph::new(RenderTarget::Screen);
        child1.add_node_id(id2);
        let id1 = pool.alloc_subgraph("child1".to_string(), child1, vec![]);
        
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(id1);
        
        let executor = RenderGraphExecutor::new();
        assert_eq!(executor.validate_with_device(&engine, &registry, &pool, &graph), Ok(()));
        let report = executor.execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None).unwrap();
        assert_eq!(report.flattened_nodes, 3);
    }

    #[test]
    fn execution_3_way_interleaved_nodes_are_ordered() {
        let mut builder = GpuEngineBuilder::new();
        builder = builder.with_required_limits(wgpu::Limits { max_compute_invocations_per_workgroup: 256, max_compute_workgroup_size_x: 256, max_compute_workgroup_size_y: 256, max_compute_workgroup_size_z: 64, ..Default::default() });
        let engine = pollster::block_on(builder.build()).unwrap();
        let mut registry = ResourceRegistry::new();
        
        let shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@vertex fn vs_main() -> @builtin(position) vec4<f32> { return vec4<f32>(0.0, 0.0, 0.0, 1.0); } @fragment fn fs_main() -> @location(0) vec4<f32> { return vec4<f32>(1.0); }")) });
        let pipeline_layout = engine.device().create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[], immediate_size: 0 });
        let draw_pipeline = engine.device().create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None, layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_main"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
        });
        registry.insert_pipeline_with_layout_descriptor(PipelineHandle(1), draw_pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![] });
        
        let compute_shader = engine.device().create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed("@compute @workgroup_size(1) fn main() {}")) });
        let compute_pipeline = engine.device().create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: None, layout: Some(&pipeline_layout), module: &compute_shader, entry_point: Some("main"), compilation_options: Default::default(), cache: None });
        registry.insert_compute_pipeline_with_layout_descriptor(ComputePipelineHandle(1), compute_pipeline, PipelineLayoutResourceDescriptor { bind_group_layout_signatures: vec![] });
        
        let buffer1 = engine.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: 4, usage: wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let buffer2 = engine.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: 4, usage: wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        registry.insert_buffer_with_descriptor(BufferHandle(1), buffer1, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_SRC }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), buffer2, BufferResourceDescriptor { size: 4, usage: wgpu::BufferUsages::COPY_DST }).unwrap();
        
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        
        let id = pool.alloc_batch(vec![DrawCommand::new(PipelineHandle(1), DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 })]);
        graph.add_node_id(id);
        let id = pool.alloc_copy_batch(vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        graph.add_node_id(id);
        let id = pool.alloc_compute_batch(vec![ComputeCommand::new(ComputePipelineHandle(1), [1, 1, 1])]);
        graph.add_node_id(id);
        
        let executor = RenderGraphExecutor::new();
        assert_eq!(executor.validate_with_device(&engine, &registry, &pool, &graph), Ok(()));
        let report = executor.execute_with_surface_checked_with_report(&engine, &registry, &mut pool, &graph, None).unwrap();
        assert_eq!(report.draw_commands, 1);
        assert_eq!(report.copy_commands, 1);
        assert_eq!(report.compute_commands, 1);
        assert_eq!(report.flattened_nodes, 3);
    }
