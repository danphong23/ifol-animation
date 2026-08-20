use crate::config::EngineConfig;
use crate::error::EngineError;
use crate::package::{EnginePackage, PackageId, PackageResolver};
use crate::provider::{ProviderManager, ResourceProvider};
use crate::registration::{CommandRegistry, RegistrationContext, RegistrationTransaction};
use crate::runtime::{EngineRuntime, RuntimeParts};
use crate::state::EngineState;

/// Fluent builder for constructing an [`EngineRuntime`].
///
/// # Contract
///
/// - The builder starts in the `Building` state.
/// - `register_package` supplies a package manifest and its contribution callback.
/// - `with_provider` registers root resource providers with topological initialization.
/// - `build()` validates all accumulated configuration, executes the registration
///   transaction atomically, compiles the ECS schedule, and transitions to `Ready`.
/// - On failure, `build()` returns a typed `EngineError` and no partial runtime is leaked.
///
/// # Example
///
/// ```rust
/// use ifol_engine::{EngineBuilder, EnginePackage, PackageManifest, PackageId, RegistrationContext,
///     Version};
///
/// struct DemoPackage(PackageManifest);
/// impl EnginePackage for DemoPackage {
///     fn manifest(&self) -> &PackageManifest { &self.0 }
///     fn register(&self, _context: &mut RegistrationContext) -> Result<(), ifol_engine::PackageError> {
///         Ok(())
///     }
/// }
///
/// let engine = EngineBuilder::new()
///     .register_package(DemoPackage(PackageManifest::new(
///         PackageId::new("pkg-demo").unwrap(), Version::new(1, 0, 0))))
///     .build()
///     .unwrap();
/// assert_eq!(engine.state(), ifol_engine::EngineState::Ready);
/// ```
pub struct EngineBuilder {
    packages: Vec<Box<dyn EnginePackage>>,
    config: EngineConfig,
    command_registry: CommandRegistry,
    provider_manager: ProviderManager,
    _state: EngineState,
}

impl EngineBuilder {
    /// Creates a new builder in the `Building` state.
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            config: EngineConfig::new(),
            command_registry: CommandRegistry::new(),
            provider_manager: ProviderManager::new(),
            _state: EngineState::Building,
        }
    }

    /// Adds one package candidate to the builder.
    ///
    /// The package is not registered immediately. Its manifest is resolved
    /// with all other candidates first; contribution registration happens in
    /// deterministic dependency order during `build`.
    pub fn register_package<P>(mut self, package: P) -> Self
    where
        P: EnginePackage + 'static,
    {
        self.packages.push(Box::new(package));
        self
    }

    /// Supplies runtime composition inputs without coupling the engine to a
    /// project file, filesystem, or serialization format.
    pub fn with_config(mut self, config: EngineConfig) -> Self {
        self.config = config;
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
    pub fn build(self) -> Result<EngineRuntime, EngineError> {
        let config = self.config;
        let mut resolver = PackageResolver::new();
        for package in &self.packages {
            resolver.add(package.manifest().clone());
        }
        let package_lock = if !config.required_packages().is_empty() {
            resolver.resolve_required(config.required_packages())?
        } else {
            resolver.resolve()?
        };

        if let Some(expected) = config.expected_lock()
            && expected != &package_lock
        {
            return Err(EngineError::BuildFailed {
                reason: "expected package lock differs from the resolved package closure".into(),
            });
        }

        let mut candidates: std::collections::BTreeMap<PackageId, Box<dyn EnginePackage>> = self
            .packages
            .into_iter()
            .map(|package| (package.manifest().id.clone(), package))
            .collect();

        let mut transaction = RegistrationTransaction::new();
        for resolved in &package_lock.packages {
            let package =
                candidates
                    .remove(&resolved.id)
                    .ok_or_else(|| EngineError::BuildFailed {
                        reason: format!(
                            "resolved package '{}' has no registered package candidate",
                            resolved.id
                        ),
                    })?;
            let package_id = resolved.id.clone();
            let mut context = RegistrationContext::new(package_id.clone());
            package
                .register(&mut context)
                .map_err(|error| EngineError::PackagePreparation {
                    package: package_id.clone(),
                    reason: error.to_string(),
                })?;
            transaction.stage(package_id, context.into_staging());
        }

        let ecs = ifol_ecs::EcsRuntime::new();
        let schemas = crate::scene::SchemaRegistry::new();
        let migrations = crate::scene::MigrationRegistry::new();
        let namespaces = config.namespaces();
        let (mut ecs, command_registry, schemas, migrations, mut provider_manager, namespaces) =
            transaction.commit(
                ecs,
                self.command_registry,
                schemas,
                migrations,
                self.provider_manager,
                namespaces,
            )?;

        // Initialize resource providers topologically with fail-closed rollback
        provider_manager.init_all(&mut ecs)?;

        Ok(EngineRuntime::from_parts(RuntimeParts {
            ecs,
            command_registry,
            provider_manager,
            package_lock,
            schemas,
            migrations,
            namespaces,
        }))
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
