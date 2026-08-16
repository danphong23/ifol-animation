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

pub(crate) fn usages_conflict(left: &ResourceUsage, right: &ResourceUsage) -> bool {
    left.resource == right.resource
        && subresources_overlap(left.subresource, right.subresource)
        && accesses_conflict(left.access, right.access)
}

fn accesses_conflict(left: ResourceAccess, right: ResourceAccess) -> bool {
    !matches!((left, right), (ResourceAccess::Read, ResourceAccess::Read))
}

fn subresources_overlap(left: ResourceSubresource, right: ResourceSubresource) -> bool {
    match (left, right) {
        (ResourceSubresource::Whole, _) | (_, ResourceSubresource::Whole) => true,
        (
            ResourceSubresource::BufferRange {
                start: left_start,
                end: left_end,
            },
            ResourceSubresource::BufferRange {
                start: right_start,
                end: right_end,
            },
        ) => left_start < right_end && right_start < left_end,
        (ResourceSubresource::BufferRange { .. }, _)
        | (_, ResourceSubresource::BufferRange { .. }) => false,
        (
            ResourceSubresource::Texture {
                mip_level: left_mip,
                array_layer: left_layer,
            },
            ResourceSubresource::TextureAspect {
                mip_level: right_mip,
                array_layer: right_layer,
                ..
            },
        )
        | (
            ResourceSubresource::TextureAspect {
                mip_level: left_mip,
                array_layer: left_layer,
                ..
            },
            ResourceSubresource::Texture {
                mip_level: right_mip,
                array_layer: right_layer,
            },
        ) => left_mip == right_mip && left_layer == right_layer,
        (
            ResourceSubresource::TextureRange {
                mip_start: left_mip_start,
                mip_end: left_mip_end,
                layer_start: left_layer_start,
                layer_end: left_layer_end,
            },
            ResourceSubresource::TextureAspectRange {
                mip_start: right_mip_start,
                mip_end: right_mip_end,
                layer_start: right_layer_start,
                layer_end: right_layer_end,
                ..
            },
        )
        | (
            ResourceSubresource::TextureAspectRange {
                mip_start: left_mip_start,
                mip_end: left_mip_end,
                layer_start: left_layer_start,
                layer_end: left_layer_end,
                ..
            },
            ResourceSubresource::TextureRange {
                mip_start: right_mip_start,
                mip_end: right_mip_end,
                layer_start: right_layer_start,
                layer_end: right_layer_end,
            },
        ) => {
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end
        }
        (
            ResourceSubresource::TextureAspect {
                mip_level: left_mip,
                array_layer: left_layer,
                aspect: left_aspect,
            },
            ResourceSubresource::TextureAspect {
                mip_level: right_mip,
                array_layer: right_layer,
                aspect: right_aspect,
            },
        ) => {
            left_mip == right_mip
                && left_layer == right_layer
                && aspects_overlap(left_aspect, right_aspect)
        }
        (
            ResourceSubresource::TextureAspectRange {
                mip_start: left_mip_start,
                mip_end: left_mip_end,
                layer_start: left_layer_start,
                layer_end: left_layer_end,
                aspect: left_aspect,
            },
            ResourceSubresource::TextureAspectRange {
                mip_start: right_mip_start,
                mip_end: right_mip_end,
                layer_start: right_layer_start,
                layer_end: right_layer_end,
                aspect: right_aspect,
            },
        ) => {
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end
                && aspects_overlap(left_aspect, right_aspect)
        }
        (
            ResourceSubresource::Texture {
                mip_level: left_mip,
                array_layer: left_layer,
            },
            ResourceSubresource::Texture {
                mip_level: right_mip,
                array_layer: right_layer,
            },
        ) => left_mip == right_mip && left_layer == right_layer,
        (
            ResourceSubresource::Texture {
                mip_level,
                array_layer,
            },
            ResourceSubresource::TextureRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
            },
        )
        | (
            ResourceSubresource::TextureRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
            },
            ResourceSubresource::Texture {
                mip_level,
                array_layer,
            },
        ) => {
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end
        }
        (
            ResourceSubresource::TextureRange {
                mip_start: left_mip_start,
                mip_end: left_mip_end,
                layer_start: left_layer_start,
                layer_end: left_layer_end,
            },
            ResourceSubresource::TextureRange {
                mip_start: right_mip_start,
                mip_end: right_mip_end,
                layer_start: right_layer_start,
                layer_end: right_layer_end,
            },
        ) => {
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end
        }
        (
            ResourceSubresource::TextureAspect {
                mip_level,
                array_layer,
                ..
            },
            ResourceSubresource::TextureAspectRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
                ..
            },
        )
        | (
            ResourceSubresource::TextureAspectRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
                ..
            },
            ResourceSubresource::TextureAspect {
                mip_level,
                array_layer,
                ..
            },
        ) => {
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end
        }
        (
            ResourceSubresource::Texture {
                mip_level,
                array_layer,
            },
            ResourceSubresource::TextureAspectRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
                ..
            },
        )
        | (
            ResourceSubresource::TextureAspectRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
                ..
            },
            ResourceSubresource::Texture {
                mip_level,
                array_layer,
            },
        ) => {
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end
        }
        _ => true,
    }
}

pub(crate) fn aspects_overlap(left: TextureAspect, right: TextureAspect) -> bool {
    matches!(
        (left, right),
        (TextureAspect::All, _) | (_, TextureAspect::All)
    ) || left == right
}

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
