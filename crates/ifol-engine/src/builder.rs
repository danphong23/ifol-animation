use crate::error::EngineError;
use crate::package::PackageId;
use crate::provider::{ProviderManager, ResourceProvider};
use crate::registration::{CommandRegistry, RegistrationContext, RegistrationTransaction};
use crate::runtime::EngineRuntime;
use crate::state::EngineState;

/// Fluent builder for constructing an [`EngineRuntime`].
///
/// # Contract
///
/// - The builder starts in the `Building` state.
/// - `with_package` allows packages to contribute components, systems, phases, etc.
/// - `with_provider` registers root resource providers with topological initialization.
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
    provider_manager: ProviderManager,
    _state: EngineState,
}

impl EngineBuilder {
    /// Creates a new builder in the `Building` state.
    pub fn new() -> Self {
        Self {
            transaction: RegistrationTransaction::new(),
            command_registry: CommandRegistry::new(),
            provider_manager: ProviderManager::new(),
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

    /// Registers a root resource provider to be topologically initialized.
    pub fn with_provider(mut self, provider: impl ResourceProvider) -> Self {
        self.provider_manager.add(Box::new(provider));
        self
    }

    /// Validates configuration, executes the registration transaction atomically,
    /// initializes resource providers, compiles the ECS schedule, and returns a ready `EngineRuntime`.
    ///
    /// If registration or provider init fails, returns `EngineError` and tears down
    /// any partial state.
    pub fn build(mut self) -> Result<EngineRuntime, EngineError> {
        let mut ecs = ifol_ecs::EcsRuntime::new();
        self.transaction
            .commit(&mut ecs, &mut self.command_registry)?;

        // Initialize resource providers topologically with fail-closed rollback
        self.provider_manager.init_all(&mut ecs)?;

        Ok(EngineRuntime::from_parts(
            ecs,
            self.command_registry,
            self.provider_manager,
        ))
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
