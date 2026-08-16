use super::ResourceSubresource;

pub(super) fn subresources_overlap(
    left: ResourceSubresource,
    right: ResourceSubresource,
) -> Option<bool> {
    match (left, right) {
        (
            ResourceSubresource::BufferRange {
                start: left_start,
                end: left_end,
            },
            ResourceSubresource::BufferRange {
                start: right_start,
                end: right_end,
            },
        ) => Some(left_start < right_end && right_start < left_end),
        (ResourceSubresource::BufferRange { .. }, _)
        | (_, ResourceSubresource::BufferRange { .. }) => Some(false),
        _ => None,
    }
}
