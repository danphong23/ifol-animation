//! Backend-facing device capability contracts.
//!
//! This module contains facts discovered from the selected `wgpu` adapter and
//! device. It deliberately does not own graph semantics or host window
//! integration. The public `api` module re-exports these types for backwards
//! compatibility.

pub mod capabilities;

pub use capabilities::{CapabilityError, GpuCapabilities};
