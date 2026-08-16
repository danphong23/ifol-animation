use crate::backend::GpuEngine;
use crate::graph::RenderTarget;
use crate::resources::handle::TextureHandle;
use crate::resources::ResourceRegistry;

pub(crate) struct TargetViews<'a> {
    pub(crate) color_view: &'a wgpu::TextureView,
    pub(crate) color_format: wgpu::TextureFormat,
    pub(crate) sample_count: u32,
    pub(crate) resolve_view: Option<&'a wgpu::TextureView>,
}

pub(crate) fn resolve_target_views<'a>(
    target: &RenderTarget,
    engine: &'a GpuEngine,
    registry: &'a ResourceRegistry,
    surface_view: Option<&'a wgpu::TextureView>,
) -> Option<TargetViews<'a>> {
    match target {
        RenderTarget::Screen => surface_view
            .zip(engine.surface_format())
            .map(|(view, format)| TargetViews {
                color_view: view,
                color_format: format,
                sample_count: 1,
                resolve_view: None,
            })
            .or_else(|| {
                registry
                    .texture(&TextureHandle(0))
                    .map(|(view, format)| TargetViews {
                        color_view: view,
                        color_format: *format,
                        sample_count: 1,
                        resolve_view: None,
                    })
            }),
        RenderTarget::Offscreen { color, .. } => {
            registry.texture(color).map(|(view, format)| TargetViews {
                color_view: view,
                color_format: *format,
                sample_count: 1,
                resolve_view: None,
            })
        }
        RenderTarget::OffscreenMsaa { color, resolve, .. } => {
            registry.texture(color).and_then(|(color_view, format)| {
                registry
                    .texture(resolve)
                    .map(|(resolve_view, _)| TargetViews {
                        color_view,
                        color_format: *format,
                        sample_count: registry
                            .texture_descriptor(color)
                            .map_or(1, |descriptor| descriptor.sample_count),
                        resolve_view: Some(resolve_view),
                    })
            })
        }
    }
}
