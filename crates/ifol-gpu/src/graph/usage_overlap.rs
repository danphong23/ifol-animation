use super::{ResourceAccess, ResourceSubresource, ResourceUsage, TextureAspect};

#[path = "buffer_overlap.rs"]
mod buffer_overlap;
#[path = "texture_overlap.rs"]
mod texture_overlap;

pub(crate) fn usages_conflict(left: &ResourceUsage, right: &ResourceUsage) -> bool {
    left.resource == right.resource
        && subresources_overlap(left.subresource, right.subresource)
        && accesses_conflict(left.access, right.access)
}

fn accesses_conflict(left: ResourceAccess, right: ResourceAccess) -> bool {
    !matches!((left, right), (ResourceAccess::Read, ResourceAccess::Read))
}

fn subresources_overlap(left: ResourceSubresource, right: ResourceSubresource) -> bool {
    if matches!(left, ResourceSubresource::Whole)
        || matches!(right, ResourceSubresource::Whole)
    {
        return true;
    }

    if let Some(overlap) = buffer_overlap::subresources_overlap(left, right) {
        return overlap;
    }

    texture_overlap::subresources_overlap(left, right).unwrap_or(true)
}

#[cfg(test)]
pub(crate) fn aspects_overlap(left: TextureAspect, right: TextureAspect) -> bool {
    texture_overlap::aspects_overlap(left, right)
}
