mod validation;
mod validation_errors;
mod validation_copy;
mod validation_compute;
mod validation_render;
mod validation_target;
mod validation_layout;
mod validation_node;
mod validation_indirect;
mod validation_texture;
pub use validation::RenderGraphValidationError;
mod bundle_key;
use validation::format_has_stencil;
mod render_bundles;
mod render_pass;
#[cfg(test)]
use render_pass::encode_draw_commands;
mod compute;
use compute::encode_compute_commands;
mod copy;
use copy::encode_copy_command;
mod nested_compile;
mod non_render;
mod prepass;
mod target_segments;
mod profiling;
mod report;
mod compiler;
mod extension;
mod counts;
mod targets;
mod flat_compile;
#[cfg(test)]
use counts::execution_counts_for_graph;
mod executor;
mod executor_profiling;
#[cfg(test)]
pub(crate) use bundle_key::bundle_cache_key;
pub use executor::RenderGraphExecutor;
pub use report::{ExecutionReport, ProfiledExecution, RenderGraphProfilingError};
#[cfg(test)]
pub(crate) use validation::bind_group_slot_index;
#[cfg(test)]
pub(crate) use validation::texture_supports_aspect;

#[cfg(test)]
#[cfg(test)]
#[path = "encoder_tests.rs"]
mod encoder_tests;
#[cfg(test)]
#[path = "executor_contract_tests.rs"]
mod executor_contract_tests;
#[cfg(test)]
#[path = "validation_contract_tests.rs"]
mod validation_contract_tests;
#[cfg(test)]
#[path = "target_tests.rs"]
mod target_tests;
#[cfg(test)]
#[path = "command_validation_tests.rs"]
mod command_validation_tests;
#[cfg(test)]
#[path = "copy_execution_tests.rs"]
mod copy_execution_tests;
#[cfg(test)]
#[path = "compute_execution_tests.rs"]
mod compute_execution_tests;
#[cfg(test)]
#[path = "dynamic_offset_tests.rs"]
mod dynamic_offset_tests;
#[cfg(test)]
#[path = "remaining_validation_tests.rs"]
mod remaining_validation_tests;
#[cfg(test)]
#[path = "execution_order_tests.rs"]
mod execution_order_tests;
