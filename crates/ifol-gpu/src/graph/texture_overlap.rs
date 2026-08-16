use super::{ResourceSubresource, TextureAspect};

pub(super) fn subresources_overlap(
    left: ResourceSubresource,
    right: ResourceSubresource,
) -> Option<bool> {
    match (left, right) {
        (ResourceSubresource::Whole, _) | (_, ResourceSubresource::Whole) => None,
        (ResourceSubresource::BufferRange { .. }, _)
        | (_, ResourceSubresource::BufferRange { .. }) => None,
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
        ) => Some(left_mip == right_mip && left_layer == right_layer),
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
        ) => Some(
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end,
        ),
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
        ) => Some(
            left_mip == right_mip
                && left_layer == right_layer
                && aspects_overlap(left_aspect, right_aspect),
        ),
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
        ) => Some(
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end
                && aspects_overlap(left_aspect, right_aspect),
        ),
        (
            ResourceSubresource::Texture {
                mip_level: left_mip,
                array_layer: left_layer,
            },
            ResourceSubresource::Texture {
                mip_level: right_mip,
                array_layer: right_layer,
            },
        ) => Some(left_mip == right_mip && left_layer == right_layer),
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
        ) => Some(
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end,
        ),
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
        ) => Some(
            left_mip_start < right_mip_end
                && right_mip_start < left_mip_end
                && left_layer_start < right_layer_end
                && right_layer_start < left_layer_end,
        ),
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
        ) => Some(
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end,
        ),
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
        ) => Some(
            mip_level >= mip_start
                && mip_level < mip_end
                && array_layer >= layer_start
                && array_layer < layer_end,
        ),
        _ => Some(true),
    }
}

pub(super) fn aspects_overlap(left: TextureAspect, right: TextureAspect) -> bool {
    matches!(
        (left, right),
        (TextureAspect::All, _) | (_, TextureAspect::All)
    ) || left == right
}
