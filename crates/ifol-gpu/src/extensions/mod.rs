//! Registration boundary for built-in and host-provided GPU extensions.
//!
//! The registry is independent from the graph kernel. An extension can be
//! discovered and versioned without making graph code know whether it
//! represents a video filter, a game effect, or another workload. Execution
//! and graph-node integration are a later layer with their own contract.

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use crate::graph::{ResourceSubresource, ResourceUsage};
#[cfg(test)]
use crate::graph::{GraphResource, ResourceAccess};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExtensionId(String);

impl ExtensionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ExtensionRegistrationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ExtensionRegistrationError::EmptyId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionDescriptor {
    pub id: ExtensionId,
    pub version: u32,
}

impl ExtensionDescriptor {
    pub fn new(id: impl Into<String>, version: u32) -> Result<Self, ExtensionRegistrationError> {
        Ok(Self { id: ExtensionId::new(id)?, version })
    }
}

/// Minimal registration contract. Encoding and usage contracts are layered on
/// top of this registry in the graph/execution integration task.
pub trait GpuExtension: Send + Sync {
    fn descriptor(&self) -> ExtensionDescriptor;
}

/// Resource contract for an operation lowered into a graph node.
pub trait ExtensionOperation: GpuExtension {
    fn resource_usages(&self) -> &[ResourceUsage];
    fn validate_operation(&self) -> Result<(), ExtensionValidationError>;
}

pub use crate::graph::{
    GraphResource as OperationResource,
    ResourceAccess as OperationAccess,
    ResourceSubresource as OperationSubresource,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionValidationError {
    #[error("extension operation contains no resource declaration for a resource-bearing operation")]
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
            ResourceSubresource::TextureRange { mip_start, mip_end, layer_start, layer_end }
                if mip_start >= mip_end || layer_start >= layer_end =>
            {
                return Err(ExtensionValidationError::InvalidResourceRange);
            }
            ResourceSubresource::TextureAspectRange { mip_start, mip_end, layer_start, layer_end, .. }
                if mip_start >= mip_end || layer_start >= layer_end =>
            {
                return Err(ExtensionValidationError::InvalidResourceRange);
            }
            _ => {}
        }
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionRegistrationError {
    #[error("extension id must not be empty")]
    EmptyId,
    #[error("extension {0:?} is already registered")]
    Duplicate(ExtensionId),
}

#[derive(Default)]
pub struct ExtensionRegistry {
    entries: HashMap<ExtensionId, Arc<dyn GpuExtension>>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        extension: Arc<dyn GpuExtension>,
    ) -> Result<(), ExtensionRegistrationError> {
        let descriptor = extension.descriptor();
        if self.entries.contains_key(&descriptor.id) {
            return Err(ExtensionRegistrationError::Duplicate(descriptor.id));
        }
        self.entries.insert(descriptor.id, extension);
        Ok(())
    }

    pub fn get(&self, id: &ExtensionId) -> Option<&Arc<dyn GpuExtension>> {
        self.entries.get(id)
    }

    pub fn contains(&self, id: &ExtensionId) -> bool {
        self.entries.contains_key(id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExtension {
        descriptor: ExtensionDescriptor,
        usages: Vec<ResourceUsage>,
    }

    impl GpuExtension for TestExtension {
        fn descriptor(&self) -> ExtensionDescriptor {
            self.descriptor.clone()
        }
    }

    impl ExtensionOperation for TestExtension {
        fn resource_usages(&self) -> &[ResourceUsage] { &self.usages }

        fn validate_operation(&self) -> Result<(), ExtensionValidationError> {
            validate_resource_usages(&self.usages)
        }
    }

    fn extension(id: &str, version: u32) -> Arc<dyn GpuExtension> {
        Arc::new(TestExtension { descriptor: ExtensionDescriptor::new(id, version).unwrap(), usages: Vec::new() })
    }

    #[test]
    fn registration_rejects_empty_id_and_duplicate() {
        assert_eq!(ExtensionId::new("  "), Err(ExtensionRegistrationError::EmptyId));

        let mut registry = ExtensionRegistry::new();
        registry.register(extension("test.filter", 1)).unwrap();
        assert_eq!(
            registry.register(extension("test.filter", 2)),
            Err(ExtensionRegistrationError::Duplicate(ExtensionId::new("test.filter").unwrap()))
        );
    }

    #[test]
    fn registration_keeps_versioned_extension_discoverable() {
        let mut registry = ExtensionRegistry::new();
        registry.register(extension("test.compute", 7)).unwrap();
        let id = ExtensionId::new("test.compute").unwrap();

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&id));
        assert_eq!(registry.get(&id).unwrap().descriptor().version, 7);
    }

    #[test]
    fn operation_contract_preserves_usage_and_rejects_invalid_ranges() {
        let valid = TestExtension {
            descriptor: ExtensionDescriptor::new("test.operation", 1).unwrap(),
            usages: vec![ResourceUsage {
                resource: GraphResource::Buffer(crate::render::BufferHandle(3)),
                access: ResourceAccess::ReadWrite,
                subresource: ResourceSubresource::BufferRange { start: 4, end: 12 },
            }],
        };
        assert_eq!(valid.resource_usages().len(), 1);
        assert_eq!(valid.validate_operation(), Ok(()));

        let invalid = [ResourceUsage {
            resource: GraphResource::Buffer(crate::render::BufferHandle(3)),
            access: ResourceAccess::Write,
            subresource: ResourceSubresource::BufferRange { start: 12, end: 12 },
        }];
        assert_eq!(validate_resource_usages(&invalid), Err(ExtensionValidationError::InvalidResourceRange));
    }
}
