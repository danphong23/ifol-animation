pub use crate::execution::*;

/// Compatibility facade for the resource layer during source migration.
pub mod handle {
    pub use crate::resources::handle::*;
}

pub mod registry {
    pub use crate::resources::registry::*;
}

/// Compatibility facade for the execution layer during source migration.
pub mod compiler {
    pub use crate::execution::*;
}

pub use crate::resources::*;

/// Compatibility facade. The graph kernel now lives at `crate::graph`; this
/// module preserves the existing `crate::render::graph::*` public path during
/// the source-tree migration.
pub mod graph {
    pub use crate::graph::*;
}

pub use crate::graph::*;
