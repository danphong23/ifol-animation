//! Root Resource Provider framework.
//!
//! Manages system and environment resources that live outside individual frames
//! (e.g. host bindings, GPU device contexts, caches, file services).
//!
//! Providers form a dependency DAG:
//! - Initialized in topological order (dependencies first).
//! - Torn down in exact reverse order during engine `shutdown()`.
//! - Fail-closed rollback: if initialization fails mid-chain, already initialized
//!   providers are torn down in reverse order before error return.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use thiserror::Error;

/// Stable identifier for a resource provider.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Errors originating from the resource provider subsystem.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProviderError {
    #[error("duplicate provider ID: '{0}'")]
    DuplicateProvider(String),

    #[error("missing resource dependency: provider '{provider}' requires '{dependency}'")]
    MissingDependency {
        provider: String,
        dependency: String,
    },

    #[error("cycle detected in provider dependency graph: {0}")]
    CycleDetected(String),

    #[error("provider '{provider}' initialization failed: {reason}")]
    InitFailed { provider: String, reason: String },

    #[error("provider '{provider}' teardown failed: {reason}")]
    TeardownFailed { provider: String, reason: String },
}

/// Trait implemented by root resource providers.
pub trait ResourceProvider: 'static + Send + Sync {
    /// Unique identifier for this provider.
    fn id(&self) -> ResourceId;

    /// List of resource dependencies that must be initialized before this provider.
    fn dependencies(&self) -> Vec<ResourceId> {
        Vec::new()
    }

    /// Initializes the resource and attaches any singleton components to the ECS world.
    fn init(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError>;

    /// Tears down the resource and cleans up.
    fn teardown(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError>;
}

/// Registry and topological manager for root resource providers.
pub struct ProviderManager {
    providers: Vec<Box<dyn ResourceProvider>>,
    initialized_order: Vec<ResourceId>,
}

impl std::fmt::Debug for ProviderManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderManager")
            .field("provider_count", &self.providers.len())
            .field("initialized_order", &self.initialized_order)
            .finish()
    }
}

impl ProviderManager {
    /// Creates an empty manager.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            initialized_order: Vec::new(),
        }
    }

    /// Adds a provider to be initialized.
    pub fn add(&mut self, provider: Box<dyn ResourceProvider>) {
        self.providers.push(provider);
    }

    /// Topologically sorts and initializes all providers into the ECS runtime.
    ///
    /// If any provider fails to initialize, all previously initialized providers
    /// are torn down in reverse order.
    pub fn init_all(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        // 1. Sort providers topologically
        let sorted_indices = self.resolve_order()?;

        // 2. Initialize in topological order
        for idx in sorted_indices {
            let provider = &mut self.providers[idx];
            let id = provider.id();

            if let Err(e) = provider.init(ecs) {
                // Fail-closed rollback
                self.rollback(ecs);
                return Err(e);
            }

            self.initialized_order.push(id);
        }

        Ok(())
    }

    /// Tears down all initialized providers in reverse topological order.
    pub fn teardown_all(&mut self, ecs: &mut ifol_ecs::EcsRuntime) -> Result<(), ProviderError> {
        let mut last_err = None;

        while let Some(id) = self.initialized_order.pop() {
            if let Some(provider) = self.providers.iter_mut().find(|p| p.id() == id)
                && let Err(e) = provider.teardown(ecs)
            {
                last_err = Some(e);
            }
        }

        if let Some(err) = last_err {
            Err(err)
        } else {
            Ok(())
        }
    }

    /// Rollback already-initialized providers during a failed init.
    fn rollback(&mut self, ecs: &mut ifol_ecs::EcsRuntime) {
        while let Some(id) = self.initialized_order.pop() {
            if let Some(provider) = self.providers.iter_mut().find(|p| p.id() == id) {
                let _ = provider.teardown(ecs);
            }
        }
    }

    /// Computes the topological sort order of providers based on their dependencies.
    fn resolve_order(&self) -> Result<Vec<usize>, ProviderError> {
        let mut id_to_index: BTreeMap<ResourceId, usize> = BTreeMap::new();
        for (i, p) in self.providers.iter().enumerate() {
            let id = p.id();
            if id_to_index.contains_key(&id) {
                return Err(ProviderError::DuplicateProvider(id.to_string()));
            }
            id_to_index.insert(id, i);
        }

        // Validate dependencies
        for p in &self.providers {
            for dep in p.dependencies() {
                if !id_to_index.contains_key(&dep) {
                    return Err(ProviderError::MissingDependency {
                        provider: p.id().to_string(),
                        dependency: dep.to_string(),
                    });
                }
            }
        }

        // Kahn's algorithm
        let mut in_degree: Vec<usize> = vec![0; self.providers.len()];
        let mut dependants: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

        for (i, p) in self.providers.iter().enumerate() {
            let deps = p.dependencies();
            in_degree[i] = deps.len();
            for dep in deps {
                let dep_idx = id_to_index[&dep];
                dependants.entry(dep_idx).or_default().push(i);
            }
        }

        let mut queue: VecDeque<usize> = in_degree
            .iter()
            .enumerate()
            .filter(|(_, deg)| **deg == 0)
            .map(|(idx, _)| idx)
            .collect();

        let mut sorted: Vec<usize> = Vec::new();

        while let Some(idx) = queue.pop_front() {
            sorted.push(idx);
            if let Some(deps) = dependants.get(&idx) {
                for &dep_idx in deps {
                    in_degree[dep_idx] -= 1;
                    if in_degree[dep_idx] == 0 {
                        queue.push_back(dep_idx);
                    }
                }
            }
        }

        if sorted.len() != self.providers.len() {
            let resolved: BTreeSet<usize> = sorted.iter().copied().collect();
            let cycle_members: Vec<String> = self
                .providers
                .iter()
                .enumerate()
                .filter(|(i, _)| !resolved.contains(i))
                .map(|(_, p)| p.id().to_string())
                .collect();
            return Err(ProviderError::CycleDetected(cycle_members.join(", ")));
        }

        Ok(sorted)
    }

    /// Returns the list of currently initialized provider IDs in activation order.
    pub fn initialized_order(&self) -> &[ResourceId] {
        &self.initialized_order
    }
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}
