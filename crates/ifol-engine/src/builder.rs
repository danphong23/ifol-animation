use crate::error::EngineError;
use crate::package::PackageId;
use crate::registration::{CommandRegistry, RegistrationContext, RegistrationTransaction};
use crate::runtime::EngineRuntime;
use crate::state::EngineState;

/// Fluent builder for constructing an [`EngineRuntime`].
///
/// # Contract
///
/// - The builder starts in the `Building` state.
/// - `with_package` allows packages to contribute components, systems, phases, etc.
/// - `build()` validates all accumulated configuration, executes the registration
///   transaction atomically, compiles the ECS schedule, and transitions to `Ready`.
/// - On failure, `build()` returns a typed `EngineError` and no partial runtime is leaked.
///
/// # Example
///
/// ```rust
/// use ifol_engine::{EngineBuilder, PackageId};
///
/// let engine = EngineBuilder::new()
///     .with_package(PackageId::new("pkg-demo").unwrap(), |ctx| {
///         // register components, systems, etc.
///     })
///     .build()
///     .unwrap();
/// assert_eq!(engine.state(), ifol_engine::EngineState::Ready);
/// ```
pub struct EngineBuilder {
    transaction: RegistrationTransaction,
    command_registry: CommandRegistry,
    _state: EngineState,
}

impl EngineBuilder {
    /// Creates a new builder in the `Building` state.
    pub fn new() -> Self {
        Self {
            transaction: RegistrationTransaction::new(),
            command_registry: CommandRegistry::new(),
            _state: EngineState::Building,
        }
    }

    /// Registers a package and collects its contributions through a [`RegistrationContext`].
    pub fn with_package<F>(mut self, package: PackageId, f: F) -> Self
    where
        F: FnOnce(&mut RegistrationContext),
    {
        let mut ctx = RegistrationContext::new(package.clone());
        f(&mut ctx);
        self.transaction.stage(package, ctx.into_staging());
        self
    }

    /// Validates configuration, executes the registration transaction atomically,
    /// compiles the ECS schedule, and returns a ready `EngineRuntime`.
    ///
    /// If registration fails (e.g. duplicate component, invalid phase graph, etc.),
    /// returns `EngineError` and discards any partial state.
    pub fn build(mut self) -> Result<EngineRuntime, EngineError> {
        let mut ecs = ifol_ecs::EcsRuntime::new();
        self.transaction
            .commit(&mut ecs, &mut self.command_registry)?;

        Ok(EngineRuntime::from_parts(ecs, self.command_registry))
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
