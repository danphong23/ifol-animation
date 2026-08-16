use crate::graph::RenderTarget;
use crate::resources::handle::TextureHandle;
use crate::resources::registry::ResourceRegistry;

use super::validation::RenderGraphValidationError;

pub(crate) fn validate_render_target(
    registry: &ResourceRegistry,
    target: &RenderTarget,
) -> Result<u32, RenderGraphValidationError> {
    match target {
        RenderTarget::Screen => Ok(1),
        RenderTarget::Offscreen {
            color,
            width,
            height,
        } => {
            if *width == 0 || *height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize {
                    width: *width,
                    height: *height,
                });
            }
            if !registry.contains_texture(color) {
                return Err(RenderGraphValidationError::MissingTexture(*color));
            }
            if let Some(descriptor) = registry.texture_descriptor(color) {
                if descriptor.width != *width || descriptor.height != *height {
                    return Err(RenderGraphValidationError::TargetSizeMismatch {
                        handle: *color,
                        width: *width,
                        height: *height,
                        actual_width: descriptor.width,
                        actual_height: descriptor.height,
                    });
                }
                let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
                if !descriptor.usage.contains(required) {
                    return Err(RenderGraphValidationError::MissingTextureUsage {
                        handle: *color,
                        required_usage: required.bits(),
                        actual_usage: descriptor.usage.bits(),
                    });
                }
                if descriptor.sample_count != 1 {
                    return Err(RenderGraphValidationError::UnsupportedSampleCount {
                        handle: *color,
                        actual: descriptor.sample_count,
                    });
                }
            }
            Ok(1)
        }
        RenderTarget::OffscreenMsaa {
            color,
            resolve,
            width,
            height,
        } => {
            if *width == 0 || *height == 0 {
                return Err(RenderGraphValidationError::InvalidTargetSize {
                    width: *width,
                    height: *height,
                });
            }
            let color_descriptor = registry
                .texture_descriptor(color)
                .ok_or(RenderGraphValidationError::MissingTexture(*color))?;
            if !registry.contains_texture(color) {
                return Err(RenderGraphValidationError::MissingTexture(*color));
            }
            if color_descriptor.width != *width || color_descriptor.height != *height {
                return Err(RenderGraphValidationError::TargetSizeMismatch {
                    handle: *color,
                    width: *width,
                    height: *height,
                    actual_width: color_descriptor.width,
                    actual_height: color_descriptor.height,
                });
            }
            if color_descriptor.sample_count <= 1 {
                return Err(RenderGraphValidationError::UnsupportedSampleCount {
                    handle: *color,
                    actual: color_descriptor.sample_count,
                });
            }
            if !color_descriptor
                .usage
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: *color,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: color_descriptor.usage.bits(),
                });
            }
            let resolve_descriptor = registry
                .texture_descriptor(resolve)
                .ok_or(RenderGraphValidationError::MissingResolveTexture(*resolve))?;
            if resolve_descriptor.width != *width || resolve_descriptor.height != *height {
                return Err(RenderGraphValidationError::ResolveSizeMismatch {
                    color_width: *width,
                    color_height: *height,
                    resolve_width: resolve_descriptor.width,
                    resolve_height: resolve_descriptor.height,
                });
            }
            if resolve_descriptor.sample_count != 1 {
                return Err(RenderGraphValidationError::InvalidResolveSampleCount {
                    handle: *resolve,
                    actual: resolve_descriptor.sample_count,
                });
            }
            if resolve_descriptor.format != color_descriptor.format {
                return Err(RenderGraphValidationError::ResolveFormatMismatch {
                    color: color_descriptor.format,
                    resolve: resolve_descriptor.format,
                });
            }
            if !resolve_descriptor
                .usage
                .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(RenderGraphValidationError::MissingTextureUsage {
                    handle: *resolve,
                    required_usage: wgpu::TextureUsages::RENDER_ATTACHMENT.bits(),
                    actual_usage: resolve_descriptor.usage.bits(),
                });
            }
            Ok(color_descriptor.sample_count)
        }
    }
}

pub(crate) fn validate_depth_stencil(
    registry: &ResourceRegistry,
    depth: Option<TextureHandle>,
    target_sample_count: u32,
) -> Result<(), RenderGraphValidationError> {
    let Some(depth) = depth else {
        return Ok(());
    };
    if !registry.contains_texture(&depth) {
        return Err(RenderGraphValidationError::MissingTexture(depth));
    }
    if let Some(descriptor) = registry.texture_descriptor(&depth) {
        let required = wgpu::TextureUsages::RENDER_ATTACHMENT;
        if !descriptor.usage.contains(required) {
            return Err(RenderGraphValidationError::MissingTextureUsage {
                handle: depth,
                required_usage: required.bits(),
                actual_usage: descriptor.usage.bits(),
            });
        }
        if descriptor.sample_count != target_sample_count {
            return Err(RenderGraphValidationError::DepthSampleCountMismatch {
                handle: depth,
                expected: target_sample_count,
                actual: descriptor.sample_count,
            });
        }
    }
    Ok(())
}
