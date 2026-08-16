use crate::resources::handle::TextureHandle;
#[cfg(test)]
use crate::resources::handle::{BufferHandle, RenderNodeId};

mod usage;
pub use usage::{GraphResource, ResourceAccess, ResourceSubresource, ResourceUsage, TextureAspect};
#[cfg(test)]
pub(crate) use usage::aspects_overlap;
mod commands;
pub use commands::{ComputeCommand, CopyCommand, DrawAction, DrawCommand};
mod nodes;
pub use nodes::{RenderNode, RenderNodePool};
mod graph;
pub use graph::{FlatRenderNode, FlatRenderPlan, GraphDependency, GraphFlattenError, RenderGraph};

/// ═══════════════════════════════════════════════════════════
/// ĐÍCH ĐẾN (RenderTarget) — "Bức tranh sẽ in lên đâu?"
/// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderTarget {
    /// In thẳng ra cửa sổ hệ điều hành (Swap Chain)
    Screen,

    /// In ra một tấm ảnh ảo trong VRAM với kích thước chính xác
    Offscreen {
        color: TextureHandle,
        width: u32,
        height: u32,
    },

    /// Render vào attachment multisample rồi resolve sang texture single-sample.
    OffscreenMsaa {
        color: TextureHandle,
        resolve: TextureHandle,
        width: u32,
        height: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::handle::{MeshHandle, PipelineHandle};

    #[test]
    fn test_render_graph_nesting() {
        let mut pool = RenderNodePool::new();

        let mut shadow_graph = RenderGraph::new(RenderTarget::Offscreen {
            color: TextureHandle(1),
            width: 2048,
            height: 2048,
        })
        .with_depth_stencil(TextureHandle(2));

        let shadow_batch_id = pool.alloc_batch(vec![DrawCommand::new(
            PipelineHandle(10),
            DrawAction::Indexed {
                mesh: MeshHandle(100),
                index_range: 0..36,
                instance_range: 0..1,
            },
        )]);
        shadow_graph.add_node_id(shadow_batch_id);

        let mut root_graph = RenderGraph::new(RenderTarget::Screen)
            .with_clear_color([0.1, 0.1, 0.1, 1.0])
            .with_depth_stencil(TextureHandle(3));

        // SubGraph Shadow Map (không có command in lên màn hình)
        let sub_id = pool.alloc_subgraph("ShadowPass", shadow_graph, vec![]);
        root_graph.add_node_id(sub_id);

        // DrawBatch chính
        let main_batch_id = pool.alloc_batch(vec![DrawCommand::new(
            PipelineHandle(20),
            DrawAction::Indexed {
                mesh: MeshHandle(200),
                index_range: 0..12,
                instance_range: 0..1,
            },
        )]);
        root_graph.add_node_id(main_batch_id);

        assert_eq!(root_graph.node_ids.len(), 2);
        match pool.get(root_graph.node_ids[0]).unwrap() {
            RenderNode::SubGraph { name, graph, commands, .. } => {
                assert_eq!(name, "ShadowPass");
                assert_eq!(graph.node_ids.len(), 1);
                assert!(commands.is_empty());
            }
            _ => panic!("Kỳ vọng Node 0 là SubGraph"),
        }
    }

    #[test]
    fn flatten_orders_child_nodes_before_subgraph_composite() {
        let mut pool = RenderNodePool::new();
        let child_batch = pool.alloc_batch(vec![]);
        let mut child_graph = RenderGraph::new(RenderTarget::Screen);
        child_graph.add_node_id(child_batch);
        let subgraph = pool.alloc_subgraph("child", child_graph, vec![]);
        let root_batch = pool.alloc_batch(vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);
        root.add_node_id(root_batch);

        let plan = root.flatten(&pool).unwrap();

        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![child_batch, subgraph, root_batch]);
        assert_eq!(plan.nodes[0].path, vec![subgraph, child_batch]);
    }

    #[test]
    fn flatten_keeps_extension_node_and_uses_its_resource_hazards() {
        let mut pool = RenderNodePool::new();
        let extension = pool.alloc_extension(
            crate::extensions::ExtensionId::new("test.filter").unwrap(),
            vec![ResourceUsage {
                resource: GraphResource::Texture(TextureHandle(9)),
                access: ResourceAccess::Write,
                subresource: ResourceSubresource::Whole,
            }],
        );
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(extension);
        graph.add_node_id(reader);
        graph.declare_resource_usage(reader, GraphResource::Texture(TextureHandle(9)), ResourceAccess::Read);

        let plan = graph.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![extension, reader]);
        assert_eq!(pool.get(extension).unwrap().extension_usages().len(), 1);
    }

    #[test]
    fn flatten_reports_missing_node() {
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(RenderNodeId(99));

        assert_eq!(
            graph.flatten(&RenderNodePool::new()),
            Err(GraphFlattenError::MissingNode(RenderNodeId(99)))
        );
    }

    #[test]
    fn flatten_applies_explicit_dependency_with_declaration_order_tiebreaker() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(second, first);

        let plan = graph.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![second, first]);
    }

    #[test]
    fn flatten_rejects_dependency_cycle() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(first, second);
        graph.add_dependency(second, first);

        assert!(matches!(graph.flatten(&pool), Err(GraphFlattenError::Cycle(_))));
    }

    #[test]
    fn direct_execution_order_uses_explicit_dependency() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(first);
        graph.add_node_id(second);
        graph.add_dependency(second, first);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![second, first]);
    }

    #[test]
    fn resource_write_then_read_creates_implicit_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_resource_usage(writer, GraphResource::Buffer(BufferHandle(1)), ResourceAccess::Write);
        graph.declare_resource_usage(reader, GraphResource::Buffer(BufferHandle(1)), ResourceAccess::Read);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![writer, reader]);
    }

    #[test]
    fn copy_commands_infer_source_read_and_destination_write_hazard() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::buffer_to_buffer(BufferHandle(1), BufferHandle(2), 4)]);
        let later_read = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_read);
        graph.declare_resource_usage(later_read, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Read);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![copy, later_read]);
    }

    #[test]
    fn texture_copy_hazard_uses_mip_and_layer_range() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::TextureToTexture {
            source: TextureHandle(1), destination: TextureHandle(2),
            source_mip_level: 0, destination_mip_level: 0,
            source_origin: [0, 0, 0], destination_origin: [0, 0, 0], extent: [4, 4, 2],
        }]);
        let later_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_writer);
        graph.declare_texture_subresource_usage(later_writer, TextureHandle(1), 1, 0, ResourceAccess::Write);
        graph.add_dependency(later_writer, copy);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![later_writer, copy]);
    }

    #[test]
    fn buffer_copy_hazard_uses_byte_range() {
        let mut pool = RenderNodePool::new();
        let copy = pool.alloc_copy_batch(vec![CopyCommand::BufferToBuffer {
            source: BufferHandle(1), destination: BufferHandle(2),
            source_offset: 0, destination_offset: 0, size: 16,
        }]);
        let later_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(copy);
        graph.add_node_id(later_writer);
        graph.declare_buffer_range_usage(later_writer, BufferHandle(1), 32, 16, ResourceAccess::Write);
        graph.add_dependency(later_writer, copy);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![later_writer, copy]);
    }

    #[test]
    fn disjoint_depth_and_stencil_aspects_do_not_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let depth_writer = pool.alloc_compute_batch(vec![]);
        let stencil_writer = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(depth_writer);
        graph.add_node_id(stencil_writer);
        graph.declare_texture_aspect_usage(depth_writer, TextureHandle(9), 0, 0, TextureAspect::DepthOnly, ResourceAccess::Write);
        graph.declare_texture_aspect_usage(stencil_writer, TextureHandle(9), 0, 0, TextureAspect::StencilOnly, ResourceAccess::Write);
        graph.add_dependency(stencil_writer, depth_writer);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![stencil_writer, depth_writer]);
    }

    #[test]
    fn all_texture_aspect_overlaps_depth_and_stencil() {
        assert!(aspects_overlap(TextureAspect::All, TextureAspect::DepthOnly));
        assert!(aspects_overlap(TextureAspect::StencilOnly, TextureAspect::All));
        assert!(!aspects_overlap(TextureAspect::DepthOnly, TextureAspect::StencilOnly));
    }

    #[test]
    fn explicit_reverse_dependency_conflicts_with_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_resource_usage(writer, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Write);
        graph.declare_resource_usage(reader, GraphResource::Buffer(BufferHandle(2)), ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert!(matches!(graph.ordered_node_ids(&pool), Err(GraphFlattenError::Cycle(_))));
    }

    #[test]
    fn disjoint_texture_subresources_do_not_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_texture_subresource_usage(writer, TextureHandle(7), 0, 0, ResourceAccess::Write);
        graph.declare_texture_subresource_usage(reader, TextureHandle(7), 1, 0, ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert_eq!(graph.ordered_node_ids(&pool).unwrap(), vec![reader, writer]);
    }

    #[test]
    fn overlapping_texture_subresources_create_hazard_edge() {
        let mut pool = RenderNodePool::new();
        let writer = pool.alloc_copy_batch(vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut graph = RenderGraph::new(RenderTarget::Screen);
        graph.add_node_id(writer);
        graph.add_node_id(reader);
        graph.declare_texture_subresource_usage(writer, TextureHandle(7), 0, 0, ResourceAccess::Write);
        graph.declare_texture_subresource_usage(reader, TextureHandle(7), 0, 0, ResourceAccess::Read);
        graph.add_dependency(reader, writer);

        assert!(matches!(graph.ordered_node_ids(&pool), Err(GraphFlattenError::Cycle(_))));
    }

    #[test]
    fn flatten_applies_hazard_between_nested_and_root_nodes() {
        let mut pool = RenderNodePool::new();
        let nested_writer = pool.alloc_copy_batch(vec![]);
        let mut child = RenderGraph::new(RenderTarget::Screen);
        child.add_node_id(nested_writer);
        child.declare_resource_usage(nested_writer, GraphResource::Texture(TextureHandle(10)), ResourceAccess::Write);
        let subgraph = pool.alloc_subgraph("producer", child, vec![]);
        let reader = pool.alloc_compute_batch(vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);
        root.add_node_id(reader);
        root.declare_resource_usage(reader, GraphResource::Texture(TextureHandle(10)), ResourceAccess::Read);

        let plan = root.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![nested_writer, subgraph, reader]);
    }

    #[test]
    fn flatten_applies_explicit_dependency_inside_nested_graph() {
        let mut pool = RenderNodePool::new();
        let first = pool.alloc_batch(vec![]);
        let second = pool.alloc_batch(vec![]);
        let mut child = RenderGraph::new(RenderTarget::Screen);
        child.add_node_id(first);
        child.add_node_id(second);
        child.add_dependency(second, first);
        let subgraph = pool.alloc_subgraph("ordered_child", child, vec![]);
        let mut root = RenderGraph::new(RenderTarget::Screen);
        root.add_node_id(subgraph);

        let plan = root.flatten(&pool).unwrap();
        assert_eq!(plan.nodes.iter().map(|node| node.node_id).collect::<Vec<_>>(), vec![second, first, subgraph]);
    }
}
