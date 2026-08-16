use super::validation::RenderGraphValidationError;
use super::RenderGraphExecutor;
use crate::api::GpuEngine;
use crate::graph::{RenderGraph, RenderNode, RenderNodePool};
use crate::resources::ResourceRegistry;

pub(crate) fn compile_nested_graphs(
    executor: &RenderGraphExecutor,
    encoder: &mut wgpu::CommandEncoder,
    engine: &GpuEngine,
    pool: &mut RenderNodePool,
    graph: &RenderGraph,
    registry: &ResourceRegistry,
    surface_view: Option<&wgpu::TextureView>,
) -> Result<(), RenderGraphValidationError> {
    for &node_id in &graph.node_ids {
        let inner_graph = if let Some(RenderNode::SubGraph { graph: inner, .. }) = pool.get(node_id)
        {
            Some(inner.clone())
        } else {
            None
        };

        if let Some(inner) = inner_graph {
            super::compiler::compile_graph(
                executor,
                encoder,
                engine,
                pool,
                &inner,
                registry,
                surface_view,
            )?;
        }
    }
    Ok(())
}
