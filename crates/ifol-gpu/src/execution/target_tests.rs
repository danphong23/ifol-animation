use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{RenderGraph, RenderNodePool, RenderTarget};
use crate::resources::{ResourceRegistry, TextureHandle, TextureResourceDescriptor};

#[test]
fn validation_rejects_graph_target_dimension_mismatch() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let texture = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("descriptor_test"),
        size: wgpu::Extent3d {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let mut registry = ResourceRegistry::new();
    registry
        .insert_texture_with_descriptor(
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
        )
        .unwrap();
    let graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(1),
        width: 64,
        height: 64,
    });

    assert_eq!(
        RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &graph),
        Err(RenderGraphValidationError::TargetSizeMismatch {
            handle: TextureHandle(1),
            width: 64,
            height: 64,
            actual_width: 128,
            actual_height: 64,
        })
    );

    let texture_without_attachment = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("usage_test"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    registry
        .insert_texture_with_descriptor(
            TextureHandle(2),
            texture_without_attachment.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                mip_level_count: 1,
                sample_count: 1,
            },
            1024,
        )
        .unwrap();
    let usage_graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(2),
        width: 64,
        height: 64,
    });
    assert!(matches!(
        RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &usage_graph),
        Err(RenderGraphValidationError::MissingTextureUsage {
            handle: TextureHandle(2),
            ..
        })
    ));

    let multisampled = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("multisampled_target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    registry
        .insert_texture_with_descriptor(
            TextureHandle(3),
            multisampled.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                mip_level_count: 1,
                sample_count: 4,
            },
            1024,
        )
        .unwrap();
    let multisample_graph = RenderGraph::new(RenderTarget::Offscreen {
        color: TextureHandle(3),
        width: 64,
        height: 64,
    });
    assert_eq!(
        RenderGraphExecutor::new().validate(&registry, &RenderNodePool::new(), &multisample_graph,),
        Err(RenderGraphValidationError::UnsupportedSampleCount {
            handle: TextureHandle(3),
            actual: 4,
        })
    );
}

#[test]
fn validation_accepts_msaa_attachment_with_single_sample_resolve() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let make_texture = |label, sample_count| {
        engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    };
    let mut registry = ResourceRegistry::new();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(10),
            make_texture("msaa_color", 4).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                mip_level_count: 1,
                sample_count: 4,
            },
            1024,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(11),
            make_texture("resolve_color", 1).create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                mip_level_count: 1,
                sample_count: 1,
            },
            1024,
        )
        .unwrap();

    let graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
        color: TextureHandle(10),
        resolve: TextureHandle(11),
        width: 64,
        height: 64,
    });
    assert!(RenderGraphExecutor::new()
        .validate(&registry, &RenderNodePool::new(), &graph)
        .is_ok());
}

#[test]
fn execute_msaa_target_with_resolve_attachment() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let make_texture = |label, sample_count| {
        engine.device().create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage,
            view_formats: &[],
        })
    };
    let color = make_texture("msaa_execute_color", 4);
    let resolve = make_texture("msaa_execute_resolve", 1);
    let mut registry = ResourceRegistry::new();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(20),
            color.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                mip_level_count: 1,
                sample_count: 4,
            },
            1024,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(21),
            resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 8,
                height: 8,
                depth_or_array_layers: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage,
                mip_level_count: 1,
                sample_count: 1,
            },
            1024,
        )
        .unwrap();
    let graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
        color: TextureHandle(20),
        resolve: TextureHandle(21),
        width: 8,
        height: 8,
    });
    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut RenderNodePool::new(), &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}

#[test]
fn execute_msaa_target_with_matching_depth_attachment() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let color_usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
    let color = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_depth_color"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: color_usage,
        view_formats: &[],
    });
    let resolve = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_depth_resolve"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: color_usage,
        view_formats: &[],
    });
    let depth = engine.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("msaa_depth_attachment"),
        size: wgpu::Extent3d {
            width: 8,
            height: 8,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 4,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24PlusStencil8,
        usage: color_usage,
        view_formats: &[],
    });
    let mut registry = ResourceRegistry::new();
    let descriptor = |format, sample_count| TextureResourceDescriptor {
        width: 8,
        height: 8,
        depth_or_array_layers: 1,
        format,
        usage: color_usage,
        mip_level_count: 1,
        sample_count,
    };
    registry
        .insert_texture_with_descriptor(
            TextureHandle(30),
            color.create_view(&wgpu::TextureViewDescriptor::default()),
            descriptor(wgpu::TextureFormat::Rgba8Unorm, 4),
            1024,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(31),
            resolve.create_view(&wgpu::TextureViewDescriptor::default()),
            descriptor(wgpu::TextureFormat::Rgba8Unorm, 1),
            1024,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(32),
            depth.create_view(&wgpu::TextureViewDescriptor::default()),
            descriptor(wgpu::TextureFormat::Depth24PlusStencil8, 4),
            1024,
        )
        .unwrap();
    let mut graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
        color: TextureHandle(30),
        resolve: TextureHandle(31),
        width: 8,
        height: 8,
    });
    graph.depth_stencil = Some(TextureHandle(32));
    let submission = RenderGraphExecutor::new()
        .execute_checked(&engine, &registry, &mut RenderNodePool::new(), &graph)
        .unwrap();
    let _ = engine.device().poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: None,
    });
}
