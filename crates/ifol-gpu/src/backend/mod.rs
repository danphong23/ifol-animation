//! Backend-facing device capability contracts.
//!
//! This module contains facts discovered from the selected `wgpu` adapter and
//! device. It deliberately does not own graph semantics or host window
//! integration. The public `api` module re-exports these types for backwards
//! compatibility.

pub mod capabilities;
pub mod builder;
pub mod engine;
mod readback;
#[cfg(feature = "image-encode")]
mod texture_save;

pub use builder::{GpuEngineBuilder, GpuError};
pub use capabilities::{CapabilityError, GpuCapabilities};
pub use engine::{GpuEngine, SurfaceResizeError};
pub use readback::{RawTextureReadback, ReadbackError, ReadbackTicket};
#[cfg(feature = "image-encode")]
pub use texture_save::TextureSaveError;
