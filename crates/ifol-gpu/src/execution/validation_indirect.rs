use crate::resources::handle::BufferHandle;
use crate::resources::ResourceRegistry;

use super::validation::RenderGraphValidationError;

pub(crate) fn validate_indirect_buffer(
    registry: &ResourceRegistry,
    handle: BufferHandle,
    offset: u64,
    size: u64,
) -> Result<(), RenderGraphValidationError> {
    let Some(buffer) = registry.buffer(&handle) else {
        return Err(RenderGraphValidationError::MissingIndirectBuffer(handle));
    };
    if !offset.is_multiple_of(4)
        || offset
            .checked_add(size)
            .is_none_or(|end| end > buffer.size())
    {
        return Err(RenderGraphValidationError::InvalidIndirectRange {
            handle,
            offset,
            size,
        });
    }
    if let Some(descriptor) = registry.buffer_descriptor(&handle) {
        let required = wgpu::BufferUsages::INDIRECT;
        if !descriptor.usage.contains(required) {
            return Err(RenderGraphValidationError::MissingIndirectBufferUsage {
                handle,
                required_usage: required.bits(),
                actual_usage: descriptor.usage.bits(),
            });
        }
    }
    Ok(())
}
