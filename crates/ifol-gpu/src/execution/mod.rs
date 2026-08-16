use thiserror::Error;
use std::sync::Arc;
use crate::api::{GpuEngine, ProfilingError, TimestampQueryPool, TimestampSpan};
use crate::extensions::ExtensionDispatchRegistry;
use crate::memory::{SubmissionId, SubmissionTracker};
use crate::graph::{RenderGraph, RenderNodePool};
use crate::resources::registry::ResourceRegistry;

mod validation;
mod validation_errors;
mod validation_copy;
mod validation_target;
mod validation_layout;
pub use validation::RenderGraphValidationError;
use validation::{format_has_stencil, validate_graph};
mod render;
use render::encode_draw_commands;
mod compute;
use compute::encode_compute_commands;
mod copy;
use copy::encode_copy_command;
mod segments;
mod profiling;
use profiling::execute_timestamped;
mod compiler;
mod extension;
mod orchestration;
use orchestration::execution_counts_for_graph;
#[cfg(test)]
pub(crate) use render::bundle_cache_key;
#[cfg(test)]
pub(crate) use validation::bind_group_slot_index;
#[cfg(test)]
pub(crate) use validation::texture_supports_aspect;

pub struct RenderGraphExecutor {
    context_key: u64,
    extension_dispatchers: Arc<ExtensionDispatchRegistry>,
}

/// Thống kê cấu trúc của một lần thực thi graph.
///
/// Đây là diagnostics hook cấp core, không giả vờ là GPU timing. Host có thể
/// dùng report để log, kiểm thử regression hoặc nối vào profiler riêng.
#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub submission: wgpu::SubmissionIndex,
    pub flattened_nodes: usize,
    pub draw_commands: usize,
    pub compute_commands: usize,
    pub copy_commands: usize,
    pub indirect_commands: usize,
    pub declared_usages: usize,
}

#[derive(Debug, Clone)]
pub struct ProfiledExecution {
    pub report: ExecutionReport,
    pub span: TimestampSpan,
    /// Submission identity used by the optional host-side completion tracker.
    /// `None` means the untracked profiling API was used.
    pub tracking_submission: Option<SubmissionId>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RenderGraphProfilingError {
    #[error(transparent)]
    Validation(#[from] RenderGraphValidationError),
    #[error(transparent)]
    Profiling(#[from] ProfilingError),
}

impl Default for RenderGraphExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphExecutor {
    pub fn new() -> Self {
        Self { context_key: 0, extension_dispatchers: Arc::new(ExtensionDispatchRegistry::new()) }
    }

    /// Gán identity ổn định cho device/viewport mà host đang dùng. Hai context
    /// khác nhau không được dùng chung bundle dù logical node giống nhau.
    pub fn with_context_key(context_key: u64) -> Self {
        Self { context_key, ..Self::new() }
    }

    pub fn with_extension_dispatchers(dispatchers: ExtensionDispatchRegistry) -> Self {
        Self { context_key: 0, extension_dispatchers: Arc::new(dispatchers) }
    }

    pub fn with_context_and_extension_dispatchers(
        context_key: u64,
        dispatchers: ExtensionDispatchRegistry,
    ) -> Self {
        Self { context_key, extension_dispatchers: Arc::new(dispatchers) }
    }

    pub fn context_key(&self) -> u64 {
        self.context_key
    }

    /// Kiểm tra graph trước khi tạo command buffer. Đây là API được khuyến nghị
    /// cho host muốn nhận lỗi typed thay vì behavior silent-skip của prototype.
    pub fn validate(
        &self,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<(), RenderGraphValidationError> {
        validate_graph(registry, pool, graph, wgpu::Limits::default().max_bind_groups, &self.extension_dispatchers)
    }

    /// Validate graph theo capability của device mà host thực sự sẽ dùng.
    /// Dùng API này khi host cần chẩn đoán trước execute mà vẫn giữ đúng
    /// `max_bind_groups` của adapter.
    pub fn validate_with_device(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<(), RenderGraphValidationError> {
        validate_graph(registry, pool, graph, engine.capabilities().max_bind_groups, &self.extension_dispatchers)
    }

    pub fn execute_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        Ok(self.execute_checked_with_report(engine, registry, pool, graph)?.submission)
    }

    pub fn execute_checked_with_report(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<ExecutionReport, RenderGraphValidationError> {
        self.execute_with_surface_checked_with_report(engine, registry, pool, graph, None)
    }

    pub fn execute_with_surface_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        Ok(self.execute_with_surface_checked_with_report(engine, registry, pool, graph, surface_view)?.submission)
    }

    pub fn execute_with_surface_checked_with_report(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<ExecutionReport, RenderGraphValidationError> {
        self.validate_with_device(engine, registry, pool, graph)?;
        let (flattened_nodes, draw_commands, compute_commands, copy_commands, indirect_commands, declared_usages) =
            execution_counts_for_graph(pool, graph)?;
        let submission = compiler::execute_unchecked(self, engine, registry, pool, graph, surface_view)?;
        Ok(ExecutionReport {
            submission,
            flattened_nodes,
            draw_commands,
            compute_commands,
            copy_commands,
            indirect_commands,
            declared_usages,
        })
    }

    /// Thực thi graph và ghi một span timestamp bao quanh toàn bộ flat/compile
    /// boundary. Đây là API opt-in; graph thông thường không chịu overhead này.
    pub fn execute_checked_with_timestamp(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        execute_timestamped(
            self,
            engine, registry, pool, graph, None, profiler, resolve_buffer, resolve_offset, None,
        )
    }

    /// Biên dịch và submit graph có profiling, đồng thời đăng ký query pool với
    /// `SubmissionTracker` trước khi submit. Host vẫn chịu trách nhiệm gọi
    /// `mark_completed` khi GPU hoàn tất submission.
    pub fn execute_checked_with_timestamp_tracked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
        tracker: &mut SubmissionTracker,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        execute_timestamped(
            self,
            engine, registry, pool, graph, None, profiler, resolve_buffer, resolve_offset, Some(tracker),
        )
    }

    pub fn execute_with_surface_checked_with_timestamp(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        execute_timestamped(
            self,
            engine, registry, pool, graph, surface_view, profiler, resolve_buffer, resolve_offset, None,
        )
    }

    /// Bản profiling có surface và lifecycle submission được tracker quản lý.
    pub fn execute_with_surface_checked_with_timestamp_tracked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
        profiler: &mut TimestampQueryPool,
        resolve_buffer: &wgpu::Buffer,
        resolve_offset: u64,
        tracker: &mut SubmissionTracker,
    ) -> Result<ProfiledExecution, RenderGraphProfilingError> {
        execute_timestamped(
            self,
            engine, registry, pool, graph, surface_view, profiler, resolve_buffer, resolve_offset, Some(tracker),
        )
    }

    /// Biên dịch RenderGraph thành các lệnh gọi WGPU và đẩy xuống GPU Queue.
    pub fn execute(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        self.execute_checked(engine, registry, pool, graph)
    }

    /// Biên dịch RenderGraph với Surface Texture View chỉ định (khi vẽ trực tiếp ra cửa sổ)
    pub fn execute_with_surface(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
        surface_view: Option<&wgpu::TextureView>,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        self.execute_with_surface_checked(engine, registry, pool, graph, surface_view)
    }



}

#[cfg(test)]
mod tests;
