use crate::extensions::ExtensionId;

use super::{ComputeCommand, CopyCommand, DrawCommand, RenderGraph, ResourceUsage};

#[derive(Debug)]
pub enum RenderNode {
    SubGraph {
        name: String,
        graph: Box<RenderGraph>,
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        use_bundle: bool,
        bundle: Option<wgpu::RenderBundle>,
        bundle_key: Option<u64>,
    },
    DrawBatch {
        commands: Vec<DrawCommand>,
        is_dirty: bool,
        use_bundle: bool,
        bundle: Option<wgpu::RenderBundle>,
        bundle_key: Option<u64>,
    },
    ComputeBatch {
        commands: Vec<ComputeCommand>,
        is_dirty: bool,
    },
    CopyBatch {
        commands: Vec<CopyCommand>,
    },
    Extension {
        extension: ExtensionId,
        usages: Vec<ResourceUsage>,
    },
}

impl RenderNode {
    pub fn new_batch(commands: Vec<DrawCommand>) -> Self {
        Self::DrawBatch {
            commands,
            is_dirty: true,
            use_bundle: true,
            bundle: None,
            bundle_key: None,
        }
    }

    pub fn new_subgraph(
        name: impl Into<String>,
        graph: RenderGraph,
        commands: Vec<DrawCommand>,
    ) -> Self {
        Self::SubGraph {
            name: name.into(),
            graph: Box::new(graph),
            commands,
            is_dirty: true,
            use_bundle: true,
            bundle: None,
            bundle_key: None,
        }
    }

    pub fn new_compute_batch(commands: Vec<ComputeCommand>) -> Self {
        Self::ComputeBatch {
            commands,
            is_dirty: true,
        }
    }

    pub fn new_extension(extension: ExtensionId, usages: Vec<ResourceUsage>) -> Self {
        Self::Extension { extension, usages }
    }

    pub fn commands(&self) -> &[DrawCommand] {
        match self {
            Self::SubGraph { commands, .. } | Self::DrawBatch { commands, .. } => commands,
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => &[],
        }
    }

    pub fn compute_commands(&self) -> &[ComputeCommand] {
        match self {
            Self::ComputeBatch { commands, .. } => commands,
            _ => &[],
        }
    }

    pub fn copy_commands(&self) -> &[CopyCommand] {
        match self {
            Self::CopyBatch { commands } => commands,
            _ => &[],
        }
    }

    pub fn extension_usages(&self) -> &[ResourceUsage] {
        match self {
            Self::Extension { usages, .. } => usages,
            _ => &[],
        }
    }

    pub fn is_dirty(&self) -> bool {
        match self {
            Self::SubGraph { is_dirty, .. }
            | Self::DrawBatch { is_dirty, .. }
            | Self::ComputeBatch { is_dirty, .. } => *is_dirty,
            Self::CopyBatch { .. } | Self::Extension { .. } => false,
        }
    }

    pub fn bundle(&self) -> Option<&wgpu::RenderBundle> {
        match self {
            Self::SubGraph { bundle, .. } | Self::DrawBatch { bundle, .. } => bundle.as_ref(),
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => None,
        }
    }

    pub fn bundle_key(&self) -> Option<u64> {
        match self {
            Self::SubGraph { bundle_key, .. } | Self::DrawBatch { bundle_key, .. } => *bundle_key,
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => None,
        }
    }

    pub fn set_bundle_key(&mut self, key: u64) {
        match self {
            Self::SubGraph { bundle_key, .. } | Self::DrawBatch { bundle_key, .. } => {
                *bundle_key = Some(key)
            }
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => {}
        }
    }

    pub fn set_use_bundle(&mut self, use_bundle: bool) {
        match self {
            Self::SubGraph {
                use_bundle: current,
                is_dirty,
                ..
            }
            | Self::DrawBatch {
                use_bundle: current,
                is_dirty,
                ..
            } => {
                *current = use_bundle;
                *is_dirty = true;
            }
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => {}
        }
    }

    pub fn use_bundle(&self) -> bool {
        match self {
            Self::SubGraph { use_bundle, .. } | Self::DrawBatch { use_bundle, .. } => *use_bundle,
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => false,
        }
    }

    pub fn sort_by_state(&mut self) {
        match self {
            Self::SubGraph {
                commands, is_dirty, ..
            }
            | Self::DrawBatch {
                commands, is_dirty, ..
            } => {
                commands.sort_by(|a, b| {
                    let pipeline = a.pipeline.0.cmp(&b.pipeline.0);
                    if pipeline != std::cmp::Ordering::Equal {
                        return pipeline;
                    }
                    let left = a.bind_groups.first().map(|group| group.1 .0).unwrap_or(0);
                    let right = b.bind_groups.first().map(|group| group.1 .0).unwrap_or(0);
                    left.cmp(&right)
                });
                *is_dirty = true;
            }
            Self::ComputeBatch { .. } | Self::CopyBatch { .. } | Self::Extension { .. } => {}
        }
    }
}
