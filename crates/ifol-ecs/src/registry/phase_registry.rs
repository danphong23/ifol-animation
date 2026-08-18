use crate::error::EcsError;
use std::collections::HashMap;
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
    /// Render contribution preparation phase.
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

    /// Returns a string representation of this phase.
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

/// Opaque sequential identifier assigned to a registered system.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemId(pub u32);

/// A node in the phase execution graph containing dependency edges and system bindings.
#[derive(Debug, Clone)]
pub struct PhaseNode {
    pub id: PhaseId,
    pub before: Vec<PhaseId>,
    pub after: Vec<PhaseId>,
    pub system_bindings: Vec<SystemId>,
}

impl PhaseNode {
    pub fn new(id: PhaseId) -> Self {
        Self {
            id,
            before: Vec::new(),
            after: Vec::new(),
            system_bindings: Vec::new(),
        }
    }
}

/// Registry managing phase nodes, topological edges, and system attachments.
#[derive(Default, Debug, Clone)]
pub struct PhaseRegistry {
    phases: HashMap<PhaseId, PhaseNode>,
    revision: u64,
}

impl PhaseRegistry {
    /// Creates a new empty `PhaseRegistry`.
    pub fn new() -> Self {
        Self {
            phases: HashMap::new(),
            revision: 0,
        }
    }

    /// Registers a new phase into the registry.
    pub fn register_phase(&mut self, id: PhaseId) -> Result<(), EcsError> {
        if self.phases.contains_key(&id) {
            return Err(EcsError::DuplicatePhase(id.to_string()));
        }
        self.phases.insert(id.clone(), PhaseNode::new(id));
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Attaches a registered system to the specified phase.
    pub fn attach_system(&mut self, phase: &PhaseId, system: SystemId) -> Result<(), EcsError> {
        let node = self
            .phases
            .get_mut(phase)
            .ok_or_else(|| EcsError::PhaseNotFound(phase.to_string()))?;
        node.system_bindings.push(system);
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Adds a directional dependency edge: `from` must execute BEFORE `to`.
    pub fn add_phase_edge(&mut self, from: &PhaseId, to: &PhaseId) -> Result<(), EcsError> {
        if !self.phases.contains_key(from) {
            return Err(EcsError::PhaseNotFound(from.to_string()));
        }
        if !self.phases.contains_key(to) {
            return Err(EcsError::PhaseNotFound(to.to_string()));
        }

        self.phases.get_mut(from).unwrap().before.push(to.clone());
        self.phases.get_mut(to).unwrap().after.push(from.clone());
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Returns the current phase node map.
    #[inline]
    pub fn phases(&self) -> &HashMap<PhaseId, PhaseNode> {
        &self.phases
    }

    /// Returns the current monotonic phase graph revision.
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Clears all phase registrations and resets revision.
    pub fn clear(&mut self) {
        self.phases.clear();
        self.revision = self.revision.wrapping_add(1);
    }
}
