use crate::api::GpuEngine;
use crate::graph::RenderNodePool;
use crate::resources::handle::RenderNodeId;
use crate::resources::ResourceRegistry;

use super::compute::encode_compute_commands;
use super::copy::encode_copy_command;
use super::extension::dispatch_extension;
use super::{RenderGraphExecutor, RenderGraphValidationError};

pub(crate) fn execute_non_render_nodes(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &RenderNodePool,
    registry: &ResourceRegistry,
    node_ids: &[RenderNodeId],
) -> Result<(), RenderGraphValidationError> {
    for &node_id in node_ids {
        let Some(node) = pool.get(node_id) else {
            return Err(RenderGraphValidationError::MissingNode(node_id));
        };
        dispatch_extension(executor, encoder, engine, registry, pool, node_id)?;
        for command in node.copy_commands() {
            encode_copy_command(encoder, registry, command)?;
        }
        encode_compute_commands(
            encoder,
            registry,
            node.compute_commands(),
            engine.capabilities().max_bind_groups,
        )?;
    }
    Ok(())
}
