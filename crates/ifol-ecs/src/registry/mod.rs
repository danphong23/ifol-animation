pub mod component_registry;
pub mod phase_registry;
pub mod system_registry;

pub use component_registry::{ComponentDescriptor, ComponentId, ComponentRegistry};
pub use phase_registry::{PhaseId, PhaseNode, PhaseRegistry, SystemId};
pub use system_registry::{SystemRegistration, SystemRegistry};
