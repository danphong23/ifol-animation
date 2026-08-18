use crate::error::EcsError;
use crate::registry::{ComponentRegistry, SystemId};
use crate::system::{AccessDescriptor, RunCondition, System};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SYSTEM_REGISTRY_ID: AtomicU64 = AtomicU64::new(1);

/// Registration metadata and implementation for a system.
pub struct SystemRegistration {
    pub(crate) id: SystemId,
    pub(crate) name: String,
    pub(crate) access: AccessDescriptor,
    pub(crate) conditions: Vec<RunCondition>,
    pub(crate) system: Box<dyn System>,
}

impl SystemRegistration {
    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn access(&self) -> &AccessDescriptor {
        &self.access
    }
}

/// Registry managing system instances, access contracts, and conditions.
pub struct SystemRegistry {
    registry_id: u64,
    registrations: Vec<SystemRegistration>,
    revision: u64,
}

impl Default for SystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemRegistry {
    /// Creates a new empty `SystemRegistry`.
    pub fn new() -> Self {
        Self {
            registry_id: NEXT_SYSTEM_REGISTRY_ID.fetch_add(1, Ordering::Relaxed),
            registrations: Vec::new(),
            revision: 0,
        }
    }

    /// Registers a system into the registry.
    pub fn register(
        &mut self,
        name: String,
        system: Box<dyn System>,
        access: AccessDescriptor,
        conditions: Vec<RunCondition>,
    ) -> Result<SystemId, EcsError> {
        if self
            .registrations
            .iter()
            .any(|registered| registered.name == name)
        {
            return Err(EcsError::DuplicateSystem(
                name,
                "system registry".to_string(),
            ));
        }
        if let Err(err) = access.validate() {
            return Err(EcsError::InvalidAccessDescriptor(name, err));
        }

        let id = SystemId::new(self.registry_id, self.registrations.len() as u32);
        let reg = SystemRegistration {
            id,
            name,
            access,
            conditions,
            system,
        };

        self.registrations.push(reg);
        self.revision = self.revision.wrapping_add(1);
        Ok(id)
    }

    /// Returns a reference to a system registration.
    #[inline]
    pub fn get(&self, id: SystemId) -> Option<&SystemRegistration> {
        (id.registry() == self.registry_id)
            .then(|| self.registrations.get(id.index()))
            .flatten()
    }

    /// Returns a mutable reference to a system registration.
    #[inline]
    pub(crate) fn get_mut(&mut self, id: SystemId) -> Option<&mut SystemRegistration> {
        (id.registry() == self.registry_id)
            .then(|| self.registrations.get_mut(id.index()))
            .flatten()
    }

    pub(crate) fn validate_components(
        &self,
        components: &ComponentRegistry,
    ) -> Result<(), EcsError> {
        for registration in &self.registrations {
            for id in registration
                .access
                .reads
                .iter()
                .chain(&registration.access.writes)
            {
                if components.descriptor(*id).is_none() {
                    return Err(EcsError::ComponentIdNotRegistered(format!("{id:?}")));
                }
            }
            for condition in &registration.conditions {
                condition.validate(components)?;
            }
        }
        Ok(())
    }

    /// Returns the current monotonic registration revision.
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the total number of registered systems.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.registrations.len()
    }

    /// Returns `true` if no systems are registered.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.registrations.is_empty()
    }
}
