use crate::error::SystemError;

/// Policy controlling how a runtime responds when a system returns `SystemError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionPolicy {
    /// Record the error and continue with the remaining systems.
    #[default]
    CollectErrors,
    /// Record the error and skip the remaining systems in the current phase.
    StopPhaseOnError,
    /// Abort the pass and return the typed error to the host immediately.
    FailFast,
}

/// Details on why a system was skipped during execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedSystem {
    /// Diagnostic name of the skipped system.
    pub system: String,
    /// Reason why the system was skipped (e.g. "Missing required `WorldRef<TestConfig>`").
    pub reason: String,
}

/// Comprehensive execution diagnostics produced by a `run_once` execution pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunReport {
    /// Monotonic execution counter incremented on every `run_once`.
    pub execution_revision: u64,
    /// Graph revision of the compiled schedule used for this pass.
    pub compiled_graph_revision: u64,
    /// Ordered list of phase names visited during this pass.
    pub phases_visited: Vec<String>,
    /// List of system names that executed successfully.
    pub systems_executed: Vec<String>,
    /// List of systems that were skipped along with specific reasons.
    pub systems_skipped: Vec<SkippedSystem>,
    /// Number of deferred commands processed during safe points.
    pub commands_processed: usize,
    /// Any structured system errors captured during execution.
    pub system_errors: Vec<(String, SystemError)>,
    /// Current structural version of the World after this pass.
    pub structural_version: u64,
    /// Total number of alive entities in the world at pass completion.
    pub entities_count: usize,
    /// Total duration of the execution pass in microseconds.
    pub duration_us: u64,
}

impl Default for RunReport {
    fn default() -> Self {
        Self {
            execution_revision: 0,
            compiled_graph_revision: 0,
            phases_visited: Vec::new(),
            systems_executed: Vec::new(),
            systems_skipped: Vec::new(),
            commands_processed: 0,
            system_errors: Vec::new(),
            structural_version: 0,
            entities_count: 1, // WORLD entity
            duration_us: 0,
        }
    }
}
