//! Report and input types for the engine step boundary.
//!
//! `StepInput` is a generic envelope that the host passes into
//! [`EngineRuntime::step`](crate::runtime::EngineRuntime::step).
//! `StepReport` is the output that the host receives back.
//!
//! Neither type contains domain-specific fields (keyboard, timeline,
//! render request). Those are injected by packages via typed input
//! providers registered during the build phase.

use ifol_ecs::RunReport;

/// Generic envelope passed by the host into each `step()`.
///
/// The envelope carries a correlation token and the engine revision
/// the host believes it is targeting, allowing the engine to detect
/// stale or out-of-order inputs.
#[derive(Debug, Clone, Default)]
pub struct StepInput {
    /// Opaque correlation ID chosen by the host for tracing.
    pub correlation_id: u64,
}

/// Comprehensive report returned by a successful `step()`.
///
/// Contains the ECS `RunReport`, engine-level revision counters,
/// and accumulated diagnostics.
#[derive(Debug, Clone)]
pub struct StepReport {
    /// The correlation ID echoed back from the input.
    pub correlation_id: u64,
    /// Engine revision after this step (monotonically increasing).
    pub engine_revision: u64,
    /// The ECS execution report for the inner `run_once()`.
    pub ecs_report: RunReport,
}

/// Report returned by `shutdown()`.
#[derive(Debug, Clone)]
pub struct ShutdownReport {
    /// Final engine revision at the time of shutdown.
    pub final_revision: u64,
    /// Whether there were any warnings during teardown.
    pub clean: bool,
}
