use super::{RenderGraphExecutor, RenderGraphValidationError};
use crate::backend::GpuEngineBuilder;
use crate::graph::{RenderGraph, RenderNodePool, RenderTarget};
use crate::resources::{ResourceRegistry, TextureHandle, TextureResourceDescriptor};

#[test]
fn validation_rejects_missing_texture_usage_for_depth() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = ResourceRegistry::new();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(99),
            engine
                .device()
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: 100,
                        height: 100,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth24Plus,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
                mip_level_count: 1,
                sample_count: 1,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
            },
            100,
        )
        .unwrap();
    let pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::Screen);
    graph.depth_stencil = Some(TextureHandle(99));

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::MissingTextureUsage {
            handle: TextureHandle(99),
            required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
            actual_usage: wgpu::TextureUsages::TEXTURE_BINDING.bits(),
        })
    );
}

#[test]
fn validation_rejects_depth_sample_count_mismatch() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut registry = ResourceRegistry::new();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(1),
            engine
                .device()
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: 100,
                        height: 100,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 4,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
                mip_level_count: 1,
                sample_count: 4,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            100,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(2),
            engine
                .device()
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: 100,
                        height: 100,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
                mip_level_count: 1,
                sample_count: 1,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            100,
        )
        .unwrap();
    registry
        .insert_texture_with_descriptor(
            TextureHandle(99),
            engine
                .device()
                .create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: 100,
                        height: 100,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth24Plus,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default()),
            TextureResourceDescriptor {
                width: 100,
                height: 100,
                depth_or_array_layers: 1,
                mip_level_count: 1,
                sample_count: 1,
                format: wgpu::TextureFormat::Depth24Plus,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            },
            100,
        )
        .unwrap();
    let pool = RenderNodePool::new();
    let mut graph = RenderGraph::new(RenderTarget::OffscreenMsaa {
        color: TextureHandle(1),
        resolve: TextureHandle(2),
        width: 100,
        height: 100,
    });
    graph.depth_stencil = Some(TextureHandle(99));

    assert_eq!(
        RenderGraphExecutor::new().validate_with_device(&engine, &registry, &pool, &graph),
        Err(RenderGraphValidationError::DepthSampleCountMismatch {
            handle: TextureHandle(99),
            expected: 4,
            actual: 1,
        })
    );
}
