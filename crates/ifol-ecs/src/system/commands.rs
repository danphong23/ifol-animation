use crate::entity::EntityId;
use crate::storage::Component;
use crate::world::World;

enum CommandAction {
    Spawn,
    Despawn(EntityId),
    Insert(EntityId, Box<dyn FnOnce(&mut World) + Send + Sync>),
    Remove(EntityId, Box<dyn FnOnce(&mut World) + Send + Sync>),
}

/// Buffer for deferred structural mutations applied at safe points.
#[derive(Default)]
pub struct Commands {
    actions: Vec<CommandAction>,
}

impl Commands {
    /// Creates a new empty `Commands` buffer.
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    /// Queues a new entity spawn operation.
    pub fn spawn(&mut self) {
        self.actions.push(CommandAction::Spawn);
    }

    /// Queues an entity despawn operation.
    pub fn despawn(&mut self, entity: EntityId) {
        self.actions.push(CommandAction::Despawn(entity));
    }

    /// Queues a component insertion on the target entity.
    pub fn insert<T: Component>(&mut self, entity: EntityId, component: T) {
        self.actions.push(CommandAction::Insert(
            entity,
            Box::new(move |world| {
                let _ = world.insert(entity, component);
            }),
        ));
    }

    /// Queues a component removal from the target entity.
    pub fn remove<T: Component>(&mut self, entity: EntityId) {
        self.actions.push(CommandAction::Remove(
            entity,
            Box::new(move |world| {
                let _ = world.remove::<T>(entity);
            }),
        ));
    }

    /// Flushes all queued commands onto the target `World`.
    ///
    /// Returns the number of commands successfully processed.
    pub fn apply(&mut self, world: &mut World) -> usize {
        let count = self.actions.len();
        let actions = std::mem::take(&mut self.actions);
        for action in actions {
            match action {
                CommandAction::Spawn => {
                    world.spawn();
                }
                CommandAction::Despawn(entity) => {
                    let _ = world.despawn(entity);
                }
                CommandAction::Insert(_entity, f) => {
                    f(world);
                }
                CommandAction::Remove(_entity, f) => {
                    f(world);
                }
            }
        }
        count
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
