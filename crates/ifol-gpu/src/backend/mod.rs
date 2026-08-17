//! Backend-facing device capability contracts.
//!
//! This module contains facts discovered from the selected `wgpu` adapter and
//! device. It deliberately does not own graph semantics or host window
//! integration. Backend types are exposed from this canonical module.

pub mod builder;
mod builder_build;
pub mod capabilities;
pub mod engine;
mod readback;

pub use builder::{GpuEngineBuilder, GpuError};
pub use capabilities::{CapabilityError, GpuCapabilities};
pub use engine::{GpuEngine, SurfaceResizeError};
pub use readback::{RawTextureReadback, ReadbackError, ReadbackTicket};
