use crate::entity::EntityId;
use crate::error::EcsError;
use crate::storage::Component;
use crate::world::World;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_COMMANDS_ID: AtomicU64 = AtomicU64::new(1);

/// Opaque handle for an entity spawned by a deferred command buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SpawnTicket {
    pub(crate) owner: u64,
    pub(crate) index: u64,
}

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
pub struct Commands {
    actions: Vec<CommandAction>,
    owner: u64,
    next_ticket: u64,
}

impl Commands {
    /// Creates a new empty command buffer.
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            owner: NEXT_COMMANDS_ID.fetch_add(1, Ordering::Relaxed),
            next_ticket: 0,
        }
    }

    /// Queues a new entity spawn and returns a ticket for same-buffer commands.
    pub fn spawn(&mut self) -> SpawnTicket {
        let ticket = SpawnTicket {
            owner: self.owner,
            index: self.next_ticket,
        };
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
                .ok_or(EcsError::UnresolvedCommandTarget(ticket.index)),
        }
    }

    /// Applies queued commands in declaration order at a safe point.
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
                    world.despawn(Self::resolve_target(target, &spawned)?)?;
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

    /// Discards all pending commands.
    pub fn clear(&mut self) {
        self.actions.clear();
    }
    /// Returns whether no commands are pending.
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

impl Default for Commands {
    fn default() -> Self {
        Self::new()
    }
}
