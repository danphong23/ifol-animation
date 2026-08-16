use crate::resources::handle::{BufferHandle, TextureHandle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphResource {
    Buffer(BufferHandle),
    Texture(TextureHandle),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceAccess {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureAspect {
    All,
    DepthOnly,
    StencilOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceSubresource {
    Whole,
    BufferRange {
        start: u64,
        end: u64,
    },
    Texture {
        mip_level: u32,
        array_layer: u32,
    },
    TextureRange {
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
    },
    TextureAspect {
        mip_level: u32,
        array_layer: u32,
        aspect: TextureAspect,
    },
    TextureAspectRange {
        mip_start: u32,
        mip_end: u32,
        layer_start: u32,
        layer_end: u32,
        aspect: TextureAspect,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceUsage {
    pub resource: GraphResource,
    pub access: ResourceAccess,
    pub subresource: ResourceSubresource,
}

#[path = "usage_overlap.rs"]
mod usage_overlap;
#[cfg(test)]
pub(crate) use usage_overlap::aspects_overlap;
pub(crate) use usage_overlap::usages_conflict;

pub(crate) fn texture_subresource_range(
    mip_level: u32,
    origin: [u32; 3],
    extent: [u32; 3],
) -> ResourceSubresource {
    let Some(layer_end) = origin[2].checked_add(extent[2]) else {
        return ResourceSubresource::Whole;
    };
    ResourceSubresource::TextureRange {
        mip_start: mip_level,
        mip_end: mip_level.saturating_add(1),
        layer_start: origin[2],
        layer_end,
    }
}

pub(crate) fn texture_aspect_subresource_range(
    mip_level: u32,
    origin: [u32; 3],
    extent: [u32; 3],
    aspect: TextureAspect,
) -> ResourceSubresource {
    let Some(layer_end) = origin[2].checked_add(extent[2]) else {
        return ResourceSubresource::Whole;
    };
    ResourceSubresource::TextureAspectRange {
        mip_start: mip_level,
        mip_end: mip_level.saturating_add(1),
        layer_start: origin[2],
        layer_end,
        aspect,
    }
}

pub(crate) fn buffer_subresource_range(offset: u64, size: u64) -> ResourceSubresource {
    if size == 0 {
        return ResourceSubresource::Whole;
    }
    let Some(end) = offset.checked_add(size) else {
        return ResourceSubresource::Whole;
    };
    ResourceSubresource::BufferRange { start: offset, end }
}
