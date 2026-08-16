//! Compatibility paths preserved while hosts migrate to the responsibility-
//! oriented `backend`, `resources`, `execution`, and `graph` modules.

/// Compatibility path for hosts that imported `api::builder` directly.
pub mod builder {
    pub use crate::backend::builder::*;
}

/// Compatibility path for hosts that imported `api::engine` directly.
pub mod engine {
    pub use crate::backend::engine::*;
}

/// Compatibility facade for the resource handle layer.
pub mod handle {
    pub use crate::resources::handle::*;
}

/// Compatibility facade for the resource registry layer.
pub mod registry {
    pub use crate::resources::*;
}

/// Compatibility facade for the execution compiler layer.
pub mod compiler {
    pub use crate::execution::*;
}

/// Compatibility path for the former `api::graph` surface.
pub mod graph {
    pub use crate::graph::*;
}
