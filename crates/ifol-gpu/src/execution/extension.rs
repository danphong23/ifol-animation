use crate::api::GpuEngine;
use crate::extensions::ExtensionExecutionContext;
use crate::graph::{RenderNode, RenderNodePool};
use crate::resources::handle::RenderNodeId;
use crate::resources::registry::ResourceRegistry;

use super::{RenderGraphExecutor, RenderGraphValidationError};

pub(crate) fn dispatch_extension(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    registry: &ResourceRegistry,
    pool: &RenderNodePool,
    node_id: RenderNodeId,
) -> Result<(), RenderGraphValidationError> {
    let Some(RenderNode::Extension { extension, usages }) = pool.get(node_id) else {
        return Ok(());
    };
    let Some(dispatcher) = executor.extension_dispatchers.get(extension) else {
        return Err(RenderGraphValidationError::UnsupportedExtension(
            extension.clone(),
        ));
    };
    dispatcher
        .encode(ExtensionExecutionContext::new(
            engine, registry, encoder, node_id, usages,
        ))
        .map_err(|error| RenderGraphValidationError::ExtensionDispatch {
            extension: extension.clone(),
            error,
        })
}
