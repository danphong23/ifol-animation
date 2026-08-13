//! Backend-facing device capability contracts.
//!
//! This module contains facts discovered from the selected `wgpu` adapter and
//! device. It deliberately does not own graph semantics or host window
//! integration. The public `api` module re-exports these types for backwards
//! compatibility.

pub mod capabilities;
pub mod builder;
pub mod engine;

pub use builder::{GpuEngineBuilder, GpuError};
pub use capabilities::{CapabilityError, GpuCapabilities};
pub use engine::{GpuEngine, ReadbackError, ReadbackTicket, SurfaceResizeError, TextureSaveError};
