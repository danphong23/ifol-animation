use std::fmt;

/// Predefined standard phases and custom phase identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PhaseId {
    /// Initial phase of the frame: input polling, system clock update.
    PreUpdate,
    /// Main simulation / animation / gameplay logic phase.
    Update,
    /// Transform propagation, physics constraints, hierarchy resolution phase.
    PostUpdate,
    /// Render contribution preparation phase (evaluating visible components).
    RenderPrepare,
    /// Graph build & command submission to GPU phase.
    RenderSubmit,
    /// Custom user-defined or feature-defined phase identifier.
    Custom(String),
}

impl PhaseId {
    /// Creates a custom phase identifier.
    pub fn custom<S: Into<String>>(name: S) -> Self {
        Self::Custom(name.into())
    }

    /// Returns a human-readable representation of this phase.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PreUpdate => "PreUpdate",
            Self::Update => "Update",
            Self::PostUpdate => "PostUpdate",
            Self::RenderPrepare => "RenderPrepare",
            Self::RenderSubmit => "RenderSubmit",
            Self::Custom(name) => name.as_str(),
        }
    }
}

impl fmt::Display for PhaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
