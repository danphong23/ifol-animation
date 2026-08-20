//! Dynamic Reconfiguration subsystem.
//!
//! Enables modifying the set of registered packages and recompiling the ECS
//! schedule at runtime without losing active state or leaving the runtime
//! in a partial faulted state if the new configuration is invalid.

mod plan;
mod transaction;

pub use plan::ReconfigurationPlan;
pub use transaction::{ReconfigurationError, ReconfigurationReport};
