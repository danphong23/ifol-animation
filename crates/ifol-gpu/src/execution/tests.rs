use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use super::{bundle_cache_key, execution_counts_for_graph, format_has_stencil, texture_supports_aspect, RenderGraphExecutor, RenderGraphProfilingError, RenderGraphValidationError};
    use super::validation::{validate_copy_range, validate_indirect_buffer};
    use crate::backend::GpuEngineBuilder;
    use crate::memory::SubmissionTracker;
    use crate::graph::{ComputeCommand, CopyCommand, DrawAction, DrawCommand, GraphResource, RenderGraph, RenderNode, RenderNodePool, RenderTarget, ResourceAccess, ResourceSubresource};
    use crate::resources::{BindGroupHandle, BindGroupResourceDescriptor, BufferHandle, BufferResourceDescriptor, ComputePipelineHandle, PipelineHandle, PipelineLayoutResourceDescriptor, RenderNodeId, ResourceRegistry, TextureHandle, TextureResourceDescriptor};

    struct CountingDispatcher {
        descriptor: crate::extensions::ExtensionDescriptor,
        calls: Arc<AtomicUsize>,
    }

    impl crate::extensions::ExtensionDispatcher for CountingDispatcher {
        fn descriptor(&self) -> crate::extensions::ExtensionDescriptor { self.descriptor.clone() }

        fn encode(
            &self,
            context: crate::extensions::ExtensionExecutionContext<'_, '_>,
        ) -> Result<(), crate::extensions::ExtensionExecutionError> {
            assert_eq!(context.usages().len(), 0);
            self.calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn extension_without_dispatch_fails_closed_before_resource_lookup() {
        let extension_id = crate::extensions::ExtensionId::new("test.unhandled").unwrap();
        let mut pool = RenderNodePool::new();
        let node = pool.alloc_extension(extension_id.clone(), Vec::new());
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(node);

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::UnsupportedExtension(extension_id))
        );
    }

    #[test]
    fn registered_extension_dispatches_once_in_no_target_path() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let extension_id = crate::extensions::ExtensionId::new("test.counting").unwrap();
        let mut dispatchers = crate::extensions::ExtensionDispatchRegistry::new();
        dispatchers.register(Arc::new(CountingDispatcher {
            descriptor: crate::extensions::ExtensionDescriptor { id: extension_id.clone(), version: 1 },
            calls: calls.clone(),
        })).unwrap();

        let mut pool = RenderNodePool::new();
        let node = pool.alloc_extension(extension_id, Vec::new());
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(node);

        RenderGraphExecutor::with_extension_dispatchers(dispatchers)
            .execute_checked(&engine, &ResourceRegistry::new(), &mut pool, &graph)
            .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn profiled_execution_is_opt_in_and_has_typed_backend_fallback() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
            return;
        };
        let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("profiling-resolve-test"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let result = RenderGraphExecutor::new().execute_checked_with_timestamp(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &RenderGraph::new(RenderTarget::Screen),
            &mut profiler,
            &resolve_buffer,
            0,
        );
        match result {
            Ok(profiled) => assert_eq!(profiled.report.flattened_nodes, 0),
            Err(RenderGraphProfilingError::Profiling(crate::api::ProfilingError::UnsupportedEncoderTimestamps)) => {}
            Err(error) => panic!("unexpected profiled execution error: {error:?}"),
        }
    }

    #[test]
    fn tracked_profiled_execution_reserves_pool_until_host_completion() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let Ok(mut profiler) = crate::api::TimestampQueryPool::new(engine.device(), 2) else {
            return;
        };
        let resolve_buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("tracked-profiling-resolve-test"),
            size: 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut tracker = SubmissionTracker::new();
        let result = RenderGraphExecutor::new().execute_checked_with_timestamp_tracked(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &RenderGraph::new(RenderTarget::Screen),
            &mut profiler,
            &resolve_buffer,
            0,
            &mut tracker,
        );
        match result {
            Ok(profiled) => {
                let submission = profiled.tracking_submission.expect("tracked API must reserve a submission");
                assert_eq!(submission, crate::memory::SubmissionId(1));
                assert_eq!(profiler.allocate_span(), Err(crate::api::ProfilingError::InFlight));
                assert!(!profiler.reset_after(&tracker).unwrap());
                tracker.mark_completed(submission);
                assert!(profiler.reset_after(&tracker).unwrap());
            }
            Err(RenderGraphProfilingError::Profiling(crate::api::ProfilingError::UnsupportedEncoderTimestamps)) => {}
            Err(error) => panic!("unexpected tracked profiling error: {error:?}"),
        }
    }

    #[test]
    fn execution_report_counts_flattened_commands_and_usages() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let draw = graph.add_batch(&mut pool, vec![DrawCommand::new(
            PipelineHandle(1),
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        )]);
        let compute = graph.add_compute_batch(&mut pool, vec![ComputeCommand::new(
            ComputePipelineHandle(2), [1, 1, 1],
        )]);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(
            BufferHandle(3), BufferHandle(4), 16,
        )]);
        graph.declare_resource_usage(draw, GraphResource::Buffer(BufferHandle(5)), ResourceAccess::Read);
        graph.declare_resource_usage(compute, GraphResource::Buffer(BufferHandle(6)), ResourceAccess::Write);

        let counts = execution_counts_for_graph(&pool, &graph).unwrap();
        assert_eq!(counts, (3, 1, 1, 1, 0, 2));
    }

    #[test]
    fn stencil_aspect_detection_is_format_specific() {
        assert!(format_has_stencil(wgpu::TextureFormat::Stencil8));
        assert!(format_has_stencil(wgpu::TextureFormat::Depth24PlusStencil8));
        assert!(format_has_stencil(wgpu::TextureFormat::Depth32FloatStencil8));
        assert!(!format_has_stencil(wgpu::TextureFormat::Depth24Plus));
        assert!(!format_has_stencil(wgpu::TextureFormat::Depth32Float));
    }

    #[test]
    fn texture_copy_aspect_support_is_format_specific() {
        use crate::graph::TextureAspect;
        assert!(texture_supports_aspect(wgpu::TextureFormat::Depth24PlusStencil8, TextureAspect::DepthOnly));
        assert!(texture_supports_aspect(wgpu::TextureFormat::Depth24PlusStencil8, TextureAspect::StencilOnly));
        assert!(texture_supports_aspect(wgpu::TextureFormat::Stencil8, TextureAspect::StencilOnly));
        assert!(!texture_supports_aspect(wgpu::TextureFormat::Rgba8Unorm, TextureAspect::DepthOnly));
        assert!(!texture_supports_aspect(wgpu::TextureFormat::Depth32Float, TextureAspect::StencilOnly));
    }

    #[test]
    fn indirect_buffer_validation_checks_alignment_range_and_usage() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let mut registry = ResourceRegistry::new();
        let buffer = engine.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("indirect_validation"), size: 64,
            usage: wgpu::BufferUsages::INDIRECT, mapped_at_creation: false,
        });
        registry.insert_buffer_with_descriptor(BufferHandle(70), buffer, BufferResourceDescriptor { size: 64, usage: wgpu::BufferUsages::INDIRECT }).unwrap();
        assert!(validate_indirect_buffer(&registry, BufferHandle(70), 0, 16).is_ok());
        assert!(matches!(validate_indirect_buffer(&registry, BufferHandle(70), 2, 16), Err(RenderGraphValidationError::InvalidIndirectRange { .. })));
        assert!(matches!(validate_indirect_buffer(&registry, BufferHandle(70), 52, 16), Err(RenderGraphValidationError::InvalidIndirectRange { .. })));
    }

    #[test]
    fn validation_rejects_missing_offscreen_target() {
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate(
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );

        assert_eq!(result, Err(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn public_execute_checked_rejects_invalid_graph_before_submit() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().execute_checked(
            &engine,
            &ResourceRegistry::new(),
            &mut RenderNodePool::new(),
            &graph,
        );

        assert_eq!(result.err(), Some(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn validate_with_device_exposes_adapter_aware_contract() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 64,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate_with_device(
            &engine,
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );
        assert_eq!(result, Err(RenderGraphValidationError::MissingTexture(TextureHandle(9))));
    }

    #[test]
    fn validation_rejects_zero_sized_target_before_resource_lookup() {
        let graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(9),
            width: 0,
            height: 64,
        });
        let result = RenderGraphExecutor::new().validate(
            &ResourceRegistry::new(),
            &RenderNodePool::new(),
            &graph,
        );

        assert_eq!(
            result,
            Err(RenderGraphValidationError::InvalidTargetSize { width: 0, height: 64 })
        );
    }

    #[test]
    fn bundle_key_changes_when_pipeline_version_changes() {
        let node = RenderNode::new_batch(vec![DrawCommand::new(
            PipelineHandle(7),
            DrawAction::Procedural { vertex_count: 3, instance_range: 0..1 },
        )]);
        let mut registry = ResourceRegistry::new();
        let first = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);
        registry.mark_pipeline_changed(PipelineHandle(7));
        let second = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);

        assert_ne!(first, second);
        let single_sample = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 0);
        let msaa = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 4, 0);
        assert_ne!(single_sample, msaa);
        let context_a = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 11);
        let context_b = bundle_cache_key(&node, &registry, wgpu::TextureFormat::Rgba8Unorm, None, 1, 22);
        assert_ne!(context_a, context_b);
    }

    #[test]
    fn validation_rejects_graph_target_dimension_mismatch() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("descriptor_test"),
            size: wgpu::Extent3d { width: 128, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(1),
            texture.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 128,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 1,
            },
            1024,
        ).unwrap();
        let graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(1), width: 64, height: 64 });

        assert_eq!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &graph),
            Err(RenderGraphValidationError::TargetSizeMismatch {
                handle: TextureHandle(1), width: 64, height: 64, actual_width: 128, actual_height: 64,
            })
        );

        let texture_without_attachment = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("usage_test"),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        registry.insert_texture_with_descriptor(
            TextureHandle(2),
            texture_without_attachment.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64, height: 64, depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1, sample_count: 1,
            },
            1024,
        ).unwrap();
        let usage_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(2), width: 64, height: 64 });
        assert!(matches!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &usage_graph),
            Err(RenderGraphValidationError::MissingTextureUsage { handle: TextureHandle(2), .. })
        ));

        let multisampled = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("multisampled_target"),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 4,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        registry.insert_texture_with_descriptor(
            TextureHandle(3),
            multisampled.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64, height: 64, depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1, sample_count: 4,
            },
            1024,
        ).unwrap();
        let multisample_graph = RenderGraph::new(RenderTarget::Offscreen { color: TextureHandle(3), width: 64, height: 64 });
        assert_eq!(
            RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &multisample_graph),
            Err(RenderGraphValidationError::UnsupportedSampleCount { handle: TextureHandle(3), actual: 4 })
        );
    }

    #[test]
    fn validation_accepts_msaa_attachment_with_single_sample_resolve() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let make_texture = |label, sample_count| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: 64, height: 64, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(10),
            make_texture("msaa_color", 4).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 64, height: 64, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 4 },
            1024,
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(11),
            make_texture("resolve_color", 1).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 64, height: 64, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 },
            1024,
        ).unwrap();

        let graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
            color: TextureHandle(10), resolve: TextureHandle(11), width: 64, height: 64,
        });
        assert!(RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &graph).is_ok());
    }

    #[test]
    fn execute_msaa_target_with_resolve_attachment() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let make_texture = |label, sample_count| engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage, view_formats: &[],
        });
        let color = make_texture("msaa_execute_color", 4);
        let resolve = make_texture("msaa_execute_resolve", 1);
        let mut registry = ResourceRegistry::new();
        registry.insert_texture_with_descriptor(
            TextureHandle(20), color.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 8, height: 8, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 4 }, 1024,
        ).unwrap();
        registry.insert_texture_with_descriptor(
            TextureHandle(21), resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor { width: 8, height: 8, depth_or_array_layers: 1, format: wgpu::TextureFormat::Rgba8Unorm, usage, mip_level_count: 1, sample_count: 1 }, 1024,
        ).unwrap();
        let graph = RenderGraph::new(RenderTarget::OffscreenMsaa { color: TextureHandle(20), resolve: TextureHandle(21), width: 8, height: 8 });
        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut RenderNodePool::new(), &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
    }

    #[test]
    fn execute_msaa_target_with_matching_depth_attachment() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let color_usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let color = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_color"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 4, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage: color_usage, view_formats: &[],
        });
        let resolve = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_resolve"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm, usage: color_usage, view_formats: &[],
        });
        let depth = engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("msaa_depth_attachment"), size: wgpu::Extent3d { width: 8, height: 8, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count: 4, dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24PlusStencil8, usage: color_usage, view_formats: &[],
        });
        let mut registry = ResourceRegistry::new();
        let descriptor = |format, sample_count| TextureResourceDescriptor {
            width: 8, height: 8, depth_or_array_layers: 1, format, usage: color_usage,
            mip_level_count: 1, sample_count,
        };
        registry.insert_texture_with_descriptor(TextureHandle(30), color.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Rgba8Unorm, 4), 1024).unwrap();
        registry.insert_texture_with_descriptor(TextureHandle(31), resolve.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Rgba8Unorm, 1), 1024).unwrap();
        registry.insert_texture_with_descriptor(TextureHandle(32), depth.create_view(&wgpu::TextureViewDescriptor::default()), descriptor(wgpu::TextureFormat::Depth24PlusStencil8, 4), 1024).unwrap();
        let mut graph = RenderGraph::new(RenderTarget::OffscreenMsaa { color: TextureHandle(30), resolve: TextureHandle(31), width: 8, height: 8 });
        graph.depth_stencil = Some(TextureHandle(32));
        let submission = RenderGraphExecutor::new().execute_checked(&engine, &registry, &mut RenderNodePool::new(), &graph).unwrap();
        let _ = engine.device().poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: None });
    }

    #[test]
    fn validation_rejects_compute_node_without_pipeline() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_compute_batch(
            &mut pool,
            vec![ComputeCommand::new(ComputePipelineHandle(42), [1, 1, 1])],
        );

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::MissingComputePipeline(ComputePipelineHandle(42)))
        );
    }

    #[test]
    fn validation_rejects_copy_node_without_buffer() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(
            &mut pool,
            vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 16)],
        );

        assert_eq!(
            RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph),
            Err(RenderGraphValidationError::MissingBuffer(BufferHandle(1)))
        );
    }

    #[test]
    fn validation_rejects_declared_usage_without_resource() {
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let node = graph.add_copy_batch(&mut pool, vec![]);
        graph.declare_resource_usage(node, GraphResource::Buffer(BufferHandle(404)), ResourceAccess::Read);
        assert_eq!(RenderGraphExecutor::new().validate(&ResourceRegistry::new(), &pool, &graph), Err(RenderGraphValidationError::MissingUsageBuffer(BufferHandle(404))));
    }

    #[test]
    fn declared_resource_usage_is_preserved_on_graph() {
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        let node = RenderNodeId(9);
        graph.declare_resource_usage(node, GraphResource::Texture(TextureHandle(7)), ResourceAccess::ReadWrite);
        assert_eq!(graph.resource_usages(&node), &[crate::graph::ResourceUsage { resource: GraphResource::Texture(TextureHandle(7)), access: ResourceAccess::ReadWrite, subresource: ResourceSubresource::Whole }]);
    }

    #[test]
    fn validation_rejects_buffer_copy_with_missing_usage_bits() {
        let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
        let source = engine.device().create_buffer(&wgpu::BufferDescriptor { label: Some("invalid_copy_source"), size: 16, usage: wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let destination = engine.device().create_buffer(&wgpu::BufferDescriptor { label: Some("invalid_copy_destination"), size: 16, usage: wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let mut registry = ResourceRegistry::new();
        registry.insert_buffer_with_descriptor(BufferHandle(1), source, BufferResourceDescriptor { size: 16, usage: wgpu::BufferUsages::COPY_DST }).unwrap();
        registry.insert_buffer_with_descriptor(BufferHandle(2), destination, BufferResourceDescriptor { size: 16, usage: wgpu::BufferUsages::COPY_SRC }).unwrap();
        let mut pool = RenderNodePool::new();
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_copy_batch(&mut pool, vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        assert_eq!(RenderGraphExecutor::new().validate(&registry, &pool, &graph), Err(RenderGraphValidationError::MissingBufferUsage { handle: BufferHandle(1), required_usage: wgpu::BufferUsages::COPY_SRC.bits(), actual_usage: wgpu::BufferUsages::COPY_DST.bits() }));
    }

    #[test]
    fn copy_range_validation_checks_overflow_and_bounds() {
        assert!(validate_copy_range(BufferHandle(1), 8, 8, 16).is_ok());
        assert!(matches!(
            validate_copy_range(BufferHandle(1), 12, 8, 16),
            Err(RenderGraphValidationError::InvalidCopyRange { .. })
        ));
        assert!(matches!(
            validate_copy_range(BufferHandle(1), u64::MAX, 1, u64::MAX),
            Err(RenderGraphValidationError::InvalidCopyRange { .. })
        ));
    }

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
