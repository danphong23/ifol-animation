//! Registration boundary for built-in and host-provided GPU extensions.
//!
//! The registries are independent from the graph kernel. An extension can be
//! discovered, versioned, and dispatched without making graph code know
//! whether it represents a video filter, a game effect, or another workload.

use crate::api::GpuEngine;
#[cfg(test)]
use crate::graph::{GraphResource, ResourceAccess, ResourceSubresource};
use crate::graph::ResourceUsage;
use crate::resources::handle::RenderNodeId;
use crate::resources::ResourceRegistry;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

mod validation;
pub use validation::{validate_resource_usages, ExtensionValidationError};

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
        Ok(Self {
            id: ExtensionId::new(id)?,
            version,
        })
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

/// Context passed to a registered extension dispatcher.
///
/// The context deliberately exposes only GPU execution primitives and the
/// resource registry. Domain payloads (shader, material, video, and so on)
/// remain owned by the host extension implementation.
pub struct ExtensionExecutionContext<'a, 'engine> {
    engine: &'a GpuEngine<'engine>,
    registry: &'a ResourceRegistry,
    encoder: &'a mut wgpu::CommandEncoder,
    node_id: RenderNodeId,
    usages: &'a [ResourceUsage],
}

impl<'a, 'engine> ExtensionExecutionContext<'a, 'engine> {
    pub(crate) fn new(
        engine: &'a GpuEngine<'engine>,
        registry: &'a ResourceRegistry,
        encoder: &'a mut wgpu::CommandEncoder,
        node_id: RenderNodeId,
        usages: &'a [ResourceUsage],
    ) -> Self {
        Self {
            engine,
            registry,
            encoder,
            node_id,
            usages,
        }
    }

    pub fn engine(&self) -> &GpuEngine<'_> {
        self.engine
    }
    pub fn registry(&self) -> &ResourceRegistry {
        self.registry
    }
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
    }
    pub fn node_id(&self) -> RenderNodeId {
        self.node_id
    }
    pub fn usages(&self) -> &[ResourceUsage] {
        self.usages
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionExecutionError {
    #[error("extension dispatcher rejected execution: {0}")]
    Rejected(String),
}

/// Host-provided encoder for one opaque graph operation.
pub trait ExtensionDispatcher: Send + Sync {
    fn descriptor(&self) -> ExtensionDescriptor;

    fn validate(&self, usages: &[ResourceUsage]) -> Result<(), ExtensionValidationError> {
        validate_resource_usages(usages)
    }

    fn encode(
        &self,
        context: ExtensionExecutionContext<'_, '_>,
    ) -> Result<(), ExtensionExecutionError>;
}

pub use crate::graph::{
    GraphResource as OperationResource, ResourceAccess as OperationAccess,
    ResourceSubresource as OperationSubresource,
};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionRegistrationError {
    #[error("extension id must not be empty")]
    EmptyId,
    #[error("extension {0:?} is already registered")]
    Duplicate(ExtensionId),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionDispatchRegistrationError {
    #[error("extension dispatcher {0:?} is already registered")]
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

#[derive(Default, Clone)]
pub struct ExtensionDispatchRegistry {
    entries: HashMap<ExtensionId, Arc<dyn ExtensionDispatcher>>,
}

impl ExtensionDispatchRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        dispatcher: Arc<dyn ExtensionDispatcher>,
    ) -> Result<(), ExtensionDispatchRegistrationError> {
        let id = dispatcher.descriptor().id;
        if self.entries.contains_key(&id) {
            return Err(ExtensionDispatchRegistrationError::Duplicate(id));
        }
        self.entries.insert(id, dispatcher);
        Ok(())
    }

    pub fn get(&self, id: &ExtensionId) -> Option<&Arc<dyn ExtensionDispatcher>> {
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
mod tests;
