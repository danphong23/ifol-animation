use crate::entity::EntityId;
use crate::error::{EcsError, SystemError};
use crate::registry::ComponentRegistry;
use crate::storage::Component;
use crate::system::AccessDescriptor;
use crate::world::World;
use std::collections::HashMap;

/// Opaque handle for an entity spawned by a deferred command buffer.
///
/// The ticket resolves only when its command buffer is applied. It may be used
/// by later commands in the same buffer, which preserves declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpawnTicket(u64);

/// Entity target accepted by deferred structural commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEntity {
    Existing(EntityId),
    Spawned(SpawnTicket),
}

impl From<EntityId> for CommandEntity {
    fn from(entity: EntityId) -> Self {
        Self::Existing(entity)
    }
}

impl From<SpawnTicket> for CommandEntity {
    fn from(ticket: SpawnTicket) -> Self {
        Self::Spawned(ticket)
    }
}

type ComponentAction = Box<dyn FnOnce(&mut World, EntityId) -> Result<(), EcsError> + Send + Sync>;

enum CommandAction {
    Spawn(SpawnTicket),
    Despawn(CommandEntity),
    Insert(CommandEntity, ComponentAction),
    Remove(CommandEntity, ComponentAction),
}

/// Buffer for deferred structural mutations applied at safe points.
#[derive(Default)]
pub struct Commands {
    actions: Vec<CommandAction>,
    next_ticket: u64,
}

impl Commands {
    /// Creates a new empty `Commands` buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues a new entity spawn and returns a ticket for same-buffer commands.
    pub fn spawn(&mut self) -> SpawnTicket {
        let ticket = SpawnTicket(self.next_ticket);
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.actions.push(CommandAction::Spawn(ticket));
        ticket
    }

    /// Queues an entity despawn operation.
    pub fn despawn(&mut self, target: impl Into<CommandEntity>) {
        self.actions.push(CommandAction::Despawn(target.into()));
    }

    /// Queues a component insertion on the target entity.
    pub fn insert<T: Component>(&mut self, target: impl Into<CommandEntity>, component: T) {
        self.actions.push(CommandAction::Insert(
            target.into(),
            Box::new(move |world, entity| world.insert(entity, component).map(|_| ())),
        ));
    }

    /// Queues a component removal from the target entity.
    pub fn remove<T: Component>(&mut self, target: impl Into<CommandEntity>) {
        self.actions.push(CommandAction::Remove(
            target.into(),
            Box::new(move |world, entity| {
                if !world.is_alive(entity) {
                    return Err(EcsError::EntityNotFound(entity));
                }
                world.remove::<T>(entity);
                Ok(())
            }),
        ));
    }

    fn resolve_target(
        target: CommandEntity,
        spawned: &HashMap<SpawnTicket, EntityId>,
    ) -> Result<EntityId, EcsError> {
        match target {
            CommandEntity::Existing(entity) => Ok(entity),
            CommandEntity::Spawned(ticket) => spawned
                .get(&ticket)
                .copied()
                .ok_or(EcsError::UnresolvedCommandTarget(ticket.0)),
        }
    }

    /// Applies queued commands in declaration order at a safe point.
    ///
    /// A command error is returned instead of being silently discarded. The
    /// remaining commands are dropped, so callers cannot accidentally replay a
    /// partially invalid buffer.
    pub fn apply(&mut self, world: &mut World) -> Result<usize, EcsError> {
        let actions = std::mem::take(&mut self.actions);
        let mut spawned = HashMap::new();
        let mut processed = 0;

        for action in actions {
            match action {
                CommandAction::Spawn(ticket) => {
                    spawned.insert(ticket, world.spawn());
                }
                CommandAction::Despawn(target) => {
                    let entity = Self::resolve_target(target, &spawned)?;
                    world.despawn(entity)?;
                }
                CommandAction::Insert(target, action) | CommandAction::Remove(target, action) => {
                    let entity = Self::resolve_target(target, &spawned)?;
                    action(world, entity)?;
                }
            }
            processed += 1;
        }

        Ok(processed)
    }

    /// Discards all pending commands, normally after a system returns an error.
    pub fn clear(&mut self) {
        self.actions.clear();
    }

    /// Returns `true` if there are no pending commands in the buffer.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns the number of pending commands.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

/// Access-checked command facade exposed to a running system.
pub struct SystemCommands<'a> {
    commands: &'a mut Commands,
    component_registry: &'a ComponentRegistry,
    access: &'a AccessDescriptor,
}

impl<'a> SystemCommands<'a> {
    pub(crate) fn new(
        commands: &'a mut Commands,
        component_registry: &'a ComponentRegistry,
        access: &'a AccessDescriptor,
    ) -> Self {
        Self {
            commands,
            component_registry,
            access,
        }
    }

    /// Queues a spawn and returns its same-buffer ticket.
    #[inline]
    pub fn spawn(&mut self) -> SpawnTicket {
        self.commands.spawn()
    }

    /// Queues a despawn. Entity lifecycle operations are structural access.
    #[inline]
    pub fn despawn(&mut self, target: impl Into<CommandEntity>) {
        self.commands.despawn(target);
    }

    fn check_write<T: Component>(&self) -> Result<(), SystemError> {
        let id = self.component_registry.get_id::<T>().ok_or_else(|| {
            SystemError::new(format!(
                "component '{}' is not registered",
                std::any::type_name::<T>()
            ))
        })?;
        if self.access.allows_write(id) {
            Ok(())
        } else {
            Err(SystemError::access_denied(
                std::any::type_name::<T>(),
                "write",
            ))
        }
    }

    /// Queues a component insertion after checking the system write contract.
    pub fn insert<T: Component>(
        &mut self,
        target: impl Into<CommandEntity>,
        component: T,
    ) -> Result<(), SystemError> {
        self.check_write::<T>()?;
        self.commands.insert(target, component);
        Ok(())
    }

    /// Queues a component removal after checking the system write contract.
    pub fn remove<T: Component>(
        &mut self,
        target: impl Into<CommandEntity>,
    ) -> Result<(), SystemError> {
        self.check_write::<T>()?;
        self.commands.remove::<T>(target);
        Ok(())
    }
}
