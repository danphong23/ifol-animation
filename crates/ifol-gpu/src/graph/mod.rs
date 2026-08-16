use crate::resources::handle::TextureHandle;
#[cfg(test)]
use crate::resources::handle::{BufferHandle, RenderNodeId};

mod usage;
#[cfg(test)]
pub(crate) use usage::aspects_overlap;
pub use usage::{GraphResource, ResourceAccess, ResourceSubresource, ResourceUsage, TextureAspect};
mod flatten;
pub use flatten::{FlatRenderNode, FlatRenderPlan, GraphDependency, GraphFlattenError};
mod flattening;
mod commands;
mod ordering;
mod resource_usage;
mod effective_usage;
pub use commands::{ComputeCommand, CopyCommand, DrawAction, DrawCommand};
mod node_pool;
mod nodes;
pub use node_pool::RenderNodePool;
pub use nodes::RenderNode;
mod graph;
pub use graph::RenderGraph;

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
mod tests;
