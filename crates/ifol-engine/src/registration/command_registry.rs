//! Generic command/query/event registry.
//!
//! Engine provides the mechanism; packages register concrete handlers.
//! No domain enums (AddShape, PlayVideo, etc.) exist in engine core.

use std::collections::HashMap;

/// Stable command identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommandId(pub String);

/// Stable query identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryId(pub String);

/// Stable event identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId(pub String);

/// Receipt returned when a command is submitted.
#[derive(Debug, Clone)]
pub struct CommandReceipt {
    pub command_id: CommandId,
    pub revision: u64,
}

/// Type-erased command handler.
pub type CommandHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>;

/// Type-erased query handler.
pub type QueryHandler = Box<dyn Fn(&[u8]) -> Result<Vec<u8>, String> + Send + Sync>;

/// Event descriptor registered by a package.
#[derive(Debug, Clone)]
pub struct EventDescriptor {
    pub id: EventId,
    pub description: String,
}

/// Registry for typed commands, queries, and events.
///
/// The registry is generic — concrete command/query/event semantics
/// are owned by packages, not by the engine.
pub struct CommandRegistry {
    commands: HashMap<CommandId, CommandHandler>,
    queries: HashMap<QueryId, QueryHandler>,
    events: HashMap<EventId, EventDescriptor>,
}

impl std::fmt::Debug for CommandRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandRegistry")
            .field("command_count", &self.commands.len())
            .field("query_count", &self.queries.len())
            .field("event_count", &self.events.len())
            .finish()
    }
}

impl CommandRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
            queries: HashMap::new(),
            events: HashMap::new(),
        }
    }

    /// Registers a command handler.
    ///
    /// Returns `Err` if the command ID is already registered.
    pub fn register_command(
        &mut self,
        id: CommandId,
        handler: CommandHandler,
    ) -> Result<(), String> {
        if self.commands.contains_key(&id) {
            return Err(format!("duplicate command ID: '{}'", id.0));
        }
        self.commands.insert(id, handler);
        Ok(())
    }

    /// Registers a query handler.
    pub fn register_query(&mut self, id: QueryId, handler: QueryHandler) -> Result<(), String> {
        if self.queries.contains_key(&id) {
            return Err(format!("duplicate query ID: '{}'", id.0));
        }
        self.queries.insert(id, handler);
        Ok(())
    }

    /// Registers an event descriptor.
    pub fn register_event(&mut self, descriptor: EventDescriptor) -> Result<(), String> {
        if self.events.contains_key(&descriptor.id) {
            return Err(format!("duplicate event ID: '{}'", descriptor.id.0));
        }
        self.events.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    /// Returns `true` if a command with the given ID is registered.
    pub fn has_command(&self, id: &CommandId) -> bool {
        self.commands.contains_key(id)
    }

    /// Returns `true` if a query with the given ID is registered.
    pub fn has_query(&self, id: &QueryId) -> bool {
        self.queries.contains_key(id)
    }

    /// Returns `true` if an event with the given ID is registered.
    pub fn has_event(&self, id: &EventId) -> bool {
        self.events.contains_key(id)
    }

    /// Returns the number of registered commands.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Returns the number of registered queries.
    pub fn query_count(&self) -> usize {
        self.queries.len()
    }

    /// Returns the number of registered events.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_find_command() {
        let mut reg = CommandRegistry::new();
        let id = CommandId("test.cmd".into());
        reg.register_command(id.clone(), Box::new(|_| Ok(vec![])))
            .unwrap();
        assert!(reg.has_command(&id));
        assert_eq!(reg.command_count(), 1);
    }

    #[test]
    fn duplicate_command_rejected() {
        let mut reg = CommandRegistry::new();
        let id = CommandId("test.cmd".into());
        reg.register_command(id.clone(), Box::new(|_| Ok(vec![])))
            .unwrap();
        assert!(reg.register_command(id, Box::new(|_| Ok(vec![]))).is_err());
    }

    #[test]
    fn register_and_find_query() {
        let mut reg = CommandRegistry::new();
        let id = QueryId("test.query".into());
        reg.register_query(id.clone(), Box::new(|_| Ok(vec![])))
            .unwrap();
        assert!(reg.has_query(&id));
    }

    #[test]
    fn register_and_find_event() {
        let mut reg = CommandRegistry::new();
        let desc = EventDescriptor {
            id: EventId("test.event".into()),
            description: "test".into(),
        };
        reg.register_event(desc).unwrap();
        assert!(reg.has_event(&EventId("test.event".into())));
    }
}
