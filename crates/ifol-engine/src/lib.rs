#![doc = include_str!("../docs/README.md")]

pub mod builder;
pub mod error;
pub mod report;
pub mod runtime;
pub mod state;

// Public re-exports
pub use builder::EngineBuilder;
pub use error::EngineError;
pub use report::{ShutdownReport, StepInput, StepReport};
pub use runtime::EngineRuntime;
pub use state::EngineState;
