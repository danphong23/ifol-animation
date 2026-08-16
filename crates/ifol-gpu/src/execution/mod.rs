mod validation;
mod validation_errors;
mod validation_copy;
mod validation_target;
mod validation_layout;
mod validation_node;
pub use validation::RenderGraphValidationError;
use validation::format_has_stencil;
mod render;
use render::encode_draw_commands;
mod compute;
use compute::encode_compute_commands;
mod copy;
use copy::encode_copy_command;
mod segments;
mod profiling;
mod compiler;
mod extension;
mod orchestration;
mod counts;
#[cfg(test)]
use counts::execution_counts_for_graph;
mod executor;
pub use executor::{
    ExecutionReport, ProfiledExecution, RenderGraphExecutor, RenderGraphProfilingError,
};
#[cfg(test)]
pub(crate) use render::bundle_cache_key;
#[cfg(test)]
pub(crate) use validation::bind_group_slot_index;
#[cfg(test)]
pub(crate) use validation::texture_supports_aspect;

#[cfg(test)]
mod tests;
