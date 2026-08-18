use crate::error::EcsError;
use crate::registry::SystemId;
use crate::system::{AccessDescriptor, RunCondition, System};

/// Registration metadata and implementation for a system.
pub struct SystemRegistration {
    pub id: SystemId,
    pub name: String,
    pub access: AccessDescriptor,
    pub conditions: Vec<RunCondition>,
    pub system: Box<dyn System>,
}

/// Registry managing system instances, access contracts, and conditions.
#[derive(Default)]
pub struct SystemRegistry {
    registrations: Vec<SystemRegistration>,
    revision: u64,
}

impl SystemRegistry {
    /// Creates a new empty `SystemRegistry`.
    pub fn new() -> Self {
        Self {
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
        if let Err(err) = access.validate() {
            return Err(EcsError::InvalidAccessDescriptor(name, err));
        }

        let id = SystemId(self.registrations.len() as u32);
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
        self.registrations.get(id.0 as usize)
    }

    /// Returns a mutable reference to a system registration.
    #[inline]
    pub fn get_mut(&mut self, id: SystemId) -> Option<&mut SystemRegistration> {
        self.registrations.get_mut(id.0 as usize)
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
