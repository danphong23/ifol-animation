//! # ifol-ecs: Pure Generic Logic & Execution Kernel
//!
//! `ifol-ecs` is a high-performance, deterministic Entity-Component-System (ECS) runtime.
//! It acts as a pure execution substrate with zero domain knowledge (completely agnostic
//! of GPU, rendering, audio, UI, or file formats).
//!
//! ## Architecture Overview
//!
//! - **EcsRuntime**: The central composition root owning World, Registries, Compiled Schedule, and Plan Caches.
//! - **Generational EntityId**: 64-bit identifier (`index: u32, generation: u32`) with slot recycling and `EntityId::WORLD`.
//! - **SparseSet Storage**: Cache-friendly contiguous storage with $O(1)$ operations and `swap_remove`.
//! - **Change Tracking**: Separate `structural_version` (topology changes) and `component_revision` / ticks (data mutations).
//! - **SystemContext & Commands**: Safe, controlled system access with deferred structural mutations flushed at safe points.
//! - **Phase DAG & Conditions**: Kahn's topological sort with cycle detection, `RunCondition` evaluation, and skip diagnostics.

pub mod entity;
pub mod error;
pub mod query;
pub mod registry;
pub mod report;
pub mod runtime;
pub mod schedule;
pub mod storage;
pub mod system;
pub mod world;

// Public re-exports
pub use entity::{EntityId, EntityManager};
pub use error::{EcsError, SystemError};
pub use query::{Query, QueryPlanCache, QueryPlanKey, With, Without, WorldQuery};
pub use registry::{
    ComponentDescriptor, ComponentId, ComponentRegistry, PhaseId, PhaseNode, PhaseRegistry,
    SystemId, SystemRegistration, SystemRegistry,
};
pub use report::{RunReport, SkippedSystem};
pub use runtime::EcsRuntime;
pub use schedule::{CompiledPhase, CompiledSchedule, PhaseGraph};
pub use storage::{AnyStorage, Component, SparseSet};
pub use system::{
    AccessDescriptor, Commands, FunctionSystem, RunCondition, System, SystemContext,
};
pub use world::World;
