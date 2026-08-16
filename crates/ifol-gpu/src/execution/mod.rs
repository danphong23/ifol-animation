mod validation;
mod validation_errors;
mod validation_copy;
mod validation_compute;
mod validation_render;
mod validation_target;
mod validation_layout;
mod validation_node;
pub use validation::RenderGraphValidationError;
use validation::format_has_stencil;
mod render_bundles;
mod render_pass;
#[cfg(test)]
use render_pass::encode_draw_commands;
mod compute;
use compute::encode_compute_commands;
mod copy;
use copy::encode_copy_command;
mod non_render;
mod prepass;
mod target_segments;
mod profiling;
mod compiler;
mod extension;
mod nested_compile;
mod counts;
mod targets;
mod flat_compile;
#[cfg(test)]
use counts::execution_counts_for_graph;
mod executor;
pub use executor::{
    ExecutionReport, ProfiledExecution, RenderGraphExecutor, RenderGraphProfilingError,
};
#[cfg(test)]
pub(crate) use render_bundles::bundle_cache_key;
#[cfg(test)]
pub(crate) use validation::bind_group_slot_index;
#[cfg(test)]
pub(crate) use validation::texture_supports_aspect;

#[cfg(test)]
mod tests;
