use super::render_graph::RenderGraph;
use super::usage::{
    buffer_subresource_range, texture_aspect_subresource_range, texture_subresource_range,
};
use super::{
    CopyCommand, DrawAction, GraphResource, RenderNodePool, RenderTarget, ResourceAccess,
    ResourceSubresource, ResourceUsage,
};
use crate::resources::handle::RenderNodeId;

impl RenderGraph {
    pub(crate) fn effective_resource_usages(
        &self,
        node_id: RenderNodeId,
        pool: &RenderNodePool,
    ) -> Vec<ResourceUsage> {
        let mut usages = self.resource_usages(&node_id).to_vec();
        if let Some(node) = pool.get(node_id) {
            usages.extend_from_slice(node.extension_usages());
            for command in node.copy_commands() {
                match command {
                    CopyCommand::BufferToBuffer {
                        source,
                        destination,
                        source_offset,
                        destination_offset,
                        size,
                    } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*source),
                            access: ResourceAccess::Read,
                            subresource: buffer_subresource_range(*source_offset, *size),
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Buffer(*destination),
                            access: ResourceAccess::Write,
                            subresource: buffer_subresource_range(*destination_offset, *size),
                        });
                    }
                    CopyCommand::TextureToTexture {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                    } => {
                        let source_subresource =
                            texture_subresource_range(*source_mip_level, *source_origin, *extent);
                        let destination_subresource = texture_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                    CopyCommand::TextureToTextureAspect {
                        source,
                        destination,
                        source_mip_level,
                        destination_mip_level,
                        source_origin,
                        destination_origin,
                        extent,
                        aspect,
                    } => {
                        let source_subresource = texture_aspect_subresource_range(
                            *source_mip_level,
                            *source_origin,
                            *extent,
                            *aspect,
                        );
                        let destination_subresource = texture_aspect_subresource_range(
                            *destination_mip_level,
                            *destination_origin,
                            *extent,
                            *aspect,
                        );
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*source),
                            access: ResourceAccess::Read,
                            subresource: source_subresource,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(*destination),
                            access: ResourceAccess::Write,
                            subresource: destination_subresource,
                        });
                    }
                }
            }
            for command in node.commands() {
                let indirect = match command.action {
                    DrawAction::Indirect { buffer, offset } => Some((buffer, offset, 16)),
                    DrawAction::IndexedIndirect { buffer, offset, .. } => {
                        Some((buffer, offset, 20))
                    }
                    _ => None,
                };
                if let Some((buffer, offset, size)) = indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, size),
                    });
                }
            }
            for command in node.compute_commands() {
                if let Some((buffer, offset)) = command.indirect {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Buffer(buffer),
                        access: ResourceAccess::Read,
                        subresource: buffer_subresource_range(offset, 12),
                    });
                }
            }
            if !node.commands().is_empty() {
                match self.target {
                    RenderTarget::Offscreen { color, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::OffscreenMsaa { color, resolve, .. } => {
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(color),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                        usages.push(ResourceUsage {
                            resource: GraphResource::Texture(resolve),
                            access: ResourceAccess::Write,
                            subresource: ResourceSubresource::Whole,
                        });
                    }
                    RenderTarget::Screen => {}
                }
                if let Some(depth) = self.depth_stencil {
                    usages.push(ResourceUsage {
                        resource: GraphResource::Texture(depth),
                        access: ResourceAccess::Write,
                        subresource: ResourceSubresource::Whole,
                    });
                }
            }
        }
        usages
    }
}
