/// Compatibility path for hosts that imported `api::builder` directly.
pub mod builder {
    pub use crate::backend::builder::*;
}

/// Compatibility path for hosts that imported `api::engine` directly.
pub mod engine {
    pub use crate::backend::engine::*;
}

pub mod profiling;

pub use crate::backend::{GpuEngineBuilder, GpuError};
pub use crate::backend::capabilities::{CapabilityError, GpuCapabilities};
pub use crate::backend::{GpuEngine, ReadbackError, ReadbackTicket, SurfaceResizeError, TextureSaveError};
pub use crate::extensions::{ExtensionDescriptor, ExtensionId, ExtensionOperation, ExtensionRegistry, ExtensionRegistrationError, ExtensionValidationError, GpuExtension};
pub use profiling::{ProfilingError, TimestampQueryPool, TimestampSpan};
