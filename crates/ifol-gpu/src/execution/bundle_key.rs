use std::hash::{Hash, Hasher};

use crate::graph::{DrawAction, RenderNode};
use crate::resources::ResourceRegistry;

pub(crate) fn bundle_cache_key(
    node: &RenderNode,
    registry: &ResourceRegistry,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    sample_count: u32,
    context_key: u64,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    color_format.hash(&mut hasher);
    depth_format.hash(&mut hasher);
    sample_count.hash(&mut hasher);
    context_key.hash(&mut hasher);
    for command in node.commands() {
        command.pipeline.0.hash(&mut hasher);
        registry
            .pipeline_version(&command.pipeline)
            .hash(&mut hasher);
        for &(slot, bind_group, ref offsets) in &command.bind_groups {
            slot.hash(&mut hasher);
            bind_group.0.hash(&mut hasher);
            registry.bind_group_version(&bind_group).hash(&mut hasher);
            offsets.hash(&mut hasher);
        }
        if let DrawAction::Indexed { mesh, .. } = command.action {
            mesh.0.hash(&mut hasher);
            registry.mesh_version(&mesh).hash(&mut hasher);
        }
    }
    hasher.finish()
}
