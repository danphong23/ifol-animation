use crate::graph::{ResourceSubresource, ResourceUsage};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionValidationError {
    #[error(
        "extension operation contains no resource declaration for a resource-bearing operation"
    )]
    MissingResourceDeclaration,
    #[error("extension operation resource usage has an invalid zero-sized range")]
    InvalidResourceRange,
}

pub fn validate_resource_usages(usages: &[ResourceUsage]) -> Result<(), ExtensionValidationError> {
    for usage in usages {
        match usage.subresource {
            ResourceSubresource::BufferRange { start, end } if start >= end => {
                return Err(ExtensionValidationError::InvalidResourceRange);
            }
            ResourceSubresource::TextureRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
            } if mip_start >= mip_end || layer_start >= layer_end => {
                return Err(ExtensionValidationError::InvalidResourceRange);
            }
            ResourceSubresource::TextureAspectRange {
                mip_start,
                mip_end,
                layer_start,
                layer_end,
                ..
            } if mip_start >= mip_end || layer_start >= layer_end => {
                return Err(ExtensionValidationError::InvalidResourceRange);
            }
            _ => {}
        }
    }
    Ok(())
}
