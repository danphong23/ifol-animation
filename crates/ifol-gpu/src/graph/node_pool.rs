use std::collections::HashMap;

use crate::extensions::ExtensionId;
use crate::resources::handle::RenderNodeId;

use super::{ComputeCommand, CopyCommand, DrawCommand, RenderGraph, RenderNode, ResourceUsage};

#[derive(Default)]
pub struct RenderNodePool {
    nodes: HashMap<RenderNodeId, RenderNode>,
    next_id: u64,
}

impl RenderNodePool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn alloc_batch(&mut self, commands: Vec<DrawCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::new_batch(commands));
        id
    }

    pub fn alloc_subgraph(
        &mut self,
        name: impl Into<String>,
        graph: RenderGraph,
        commands: Vec<DrawCommand>,
    ) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes
            .insert(id, RenderNode::new_subgraph(name, graph, commands));
        id
    }

    pub fn alloc_compute_batch(&mut self, commands: Vec<ComputeCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes
            .insert(id, RenderNode::new_compute_batch(commands));
        id
    }

    pub fn alloc_copy_batch(&mut self, commands: Vec<CopyCommand>) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes.insert(id, RenderNode::CopyBatch { commands });
        id
    }

    pub fn alloc_extension(
        &mut self,
        extension: ExtensionId,
        usages: Vec<ResourceUsage>,
    ) -> RenderNodeId {
        self.next_id += 1;
        let id = RenderNodeId(self.next_id);
        self.nodes
            .insert(id, RenderNode::new_extension(extension, usages));
        id
    }

    pub fn get(&self, id: RenderNodeId) -> Option<&RenderNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: RenderNodeId) -> Option<&mut RenderNode> {
        self.nodes.get_mut(&id)
    }

    pub fn update_commands(&mut self, id: RenderNodeId, commands: Vec<DrawCommand>) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            match node {
                RenderNode::DrawBatch {
                    commands: current,
                    is_dirty,
                    bundle,
                    bundle_key,
                    ..
                }
                | RenderNode::SubGraph {
                    commands: current,
                    is_dirty,
                    bundle,
                    bundle_key,
                    ..
                } => {
                    *current = commands;
                    *is_dirty = true;
                    *bundle = None;
                    *bundle_key = None;
                }
                RenderNode::ComputeBatch { .. }
                | RenderNode::CopyBatch { .. }
                | RenderNode::Extension { .. } => return false,
            }
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self, id: RenderNodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            match node {
                RenderNode::DrawBatch {
                    is_dirty,
                    bundle,
                    bundle_key,
                    ..
                }
                | RenderNode::SubGraph {
                    is_dirty,
                    bundle,
                    bundle_key,
                    ..
                } => {
                    *is_dirty = true;
                    *bundle = None;
                    *bundle_key = None;
                }
                RenderNode::ComputeBatch { is_dirty, .. } => *is_dirty = true,
                RenderNode::CopyBatch { .. } | RenderNode::Extension { .. } => {}
            }
        }
    }

    pub fn remove(&mut self, id: RenderNodeId) -> Option<RenderNode> {
        self.nodes.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
