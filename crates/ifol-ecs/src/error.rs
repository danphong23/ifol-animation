use crate::entity::EntityId;
use thiserror::Error;

/// Structured error produced by systems during execution.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct SystemError {
    pub message: String,
    pub code: Option<i32>,
}

impl SystemError {
    /// Creates a new system error with a descriptive message.
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            code: None,
        }
    }

    /// Creates a new system error with a specific error code.
    pub fn with_code<S: Into<String>>(message: S, code: i32) -> Self {
        Self {
            message: message.into(),
            code: Some(code),
        }
    }
}

/// Typed, fail-closed errors returned by `ifol-ecs`.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum EcsError {
    #[error("entity {0:?} does not exist or is dead (generation mismatch)")]
    EntityNotFound(EntityId),

    #[error("entity {0:?} is forged or invalid (slot is currently free)")]
    ForgedEntityId(EntityId),

    #[error("component type '{0}' has not been registered in the component registry")]
    ComponentNotRegistered(&'static str),

    #[error("component of type '{component}' was not found on entity {entity:?}")]
    ComponentNotFound {
        entity: EntityId,
        component: &'static str,
    },

    #[error("required singleton component '{0}' was not found on WORLD_ENTITY")]
    SingletonNotFound(&'static str),

    #[error("phase '{0}' was not found in the phase registry")]
    PhaseNotFound(String),

    #[error("system '{0}' was not found in the system registry")]
    SystemNotFound(String),

    #[error("component type '{0}' is already registered")]
    DuplicateComponent(&'static str),

    #[error("phase '{0}' is already registered")]
    DuplicatePhase(String),

    #[error("system '{0}' is already registered in phase '{1}'")]
    DuplicateSystem(String, String),

    #[error("phase cycle detected in schedule: {0}")]
    PhaseCycleDetected(String),

    #[error("phase '{phase}' depends on unknown phase '{dependency}'")]
    MissingPhaseDependency {
        phase: String,
        dependency: String,
    },

    #[error("invalid access descriptor for system '{0}': {1}")]
    InvalidAccessDescriptor(String, &'static str),

    #[error("resource borrow conflict: {0}")]
    BorrowConflict(&'static str),

    #[error("system '{system}' failed during execution: {error}")]
    SystemExecutionFailed {
        system: String,
        error: SystemError,
    },

    #[error("runtime has not been compiled or plan is stale (graph revision changed)")]
    ScheduleNotCompiled,
}
