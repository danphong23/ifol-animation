pub mod handle;
pub mod registry;
pub mod compiler;

pub use handle::*;
pub use registry::*;
pub use compiler::*;

/// Compatibility facade. The graph kernel now lives at `crate::graph`; this
/// module preserves the existing `crate::render::graph::*` public path during
/// the source-tree migration.
pub mod graph {
    pub use crate::graph::*;
}

pub use crate::graph::*;
