//! Compatibility paths preserved while hosts migrate to responsibility-oriented modules.

/// Compatibility facade for the resource handle layer.
pub mod handle {
    pub use crate::resources::handle::*;
}

/// Compatibility facade for the resource registry layer.
pub mod registry {
    pub use crate::resources::*;
}

/// Compatibility facade for the execution layer during source migration.
pub mod compiler {
    pub use crate::execution::*;
}

/// Compatibility facade. The graph kernel now lives at `crate::graph`; this
/// module preserves the existing `crate::render::graph::*` public path during
/// the source-tree migration.
pub mod graph {
    pub use crate::graph::*;
}
