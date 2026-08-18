use crate::error::EcsError;
use std::collections::HashMap;
use std::fmt;

/// Opaque, domain-neutral identifier for an execution phase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PhaseId(String);

impl PhaseId {
    /// Creates a phase identifier from a stable name.
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self(name.into())
    }

    /// Returns a string representation of this phase.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PhaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Opaque sequential identifier assigned to a registered system.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SystemId(u32);

impl SystemId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }

    pub(crate) const fn index(self) -> usize {
        self.0 as usize
    }
}

/// A node in the phase execution graph containing dependency edges and system bindings.
#[derive(Debug, Clone)]
pub struct PhaseNode {
    id: PhaseId,
    before: Vec<PhaseId>,
    system_bindings: Vec<SystemId>,
}

impl PhaseNode {
    pub fn new(id: PhaseId) -> Self {
        Self {
            id,
            before: Vec::new(),
            system_bindings: Vec::new(),
        }
    }

    pub fn id(&self) -> &PhaseId {
        &self.id
    }

    pub(crate) fn before(&self) -> &[PhaseId] {
        &self.before
    }

    pub(crate) fn system_bindings(&self) -> &[SystemId] {
        &self.system_bindings
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
        if id.as_str().is_empty() {
            return Err(EcsError::InvalidPhaseId);
        }
        if self.phases.contains_key(&id) {
            return Err(EcsError::DuplicatePhase(id.to_string()));
        }
        self.phases.insert(id.clone(), PhaseNode::new(id));
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Attaches a registered system to the specified phase.
    pub(crate) fn attach_system(
        &mut self,
        phase: &PhaseId,
        system: SystemId,
    ) -> Result<(), EcsError> {
        let node = self
            .phases
            .get_mut(phase)
            .ok_or_else(|| EcsError::PhaseNotFound(phase.to_string()))?;
        if node.system_bindings.contains(&system) {
            return Err(EcsError::DuplicateSystemBinding {
                phase: phase.to_string(),
                system: format!("{system:?}"),
            });
        }
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

        let from_node = self.phases.get_mut(from).expect("phase existence checked");
        if from_node.before.contains(to) {
            return Err(EcsError::DuplicatePhaseEdge {
                from: from.to_string(),
                to: to.to_string(),
            });
        }
        from_node.before.push(to.clone());
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    /// Returns the current phase node map.
    #[inline]
    pub(crate) fn phases(&self) -> &HashMap<PhaseId, PhaseNode> {
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
