use crate::api::TimestampQueryPool;
use crate::backend::GpuEngine;
use crate::extensions::ExtensionDispatchRegistry;
use crate::graph::{RenderGraph, RenderNodePool};
use crate::memory::SubmissionTracker;
use crate::resources::ResourceRegistry;
use std::sync::Arc;

use super::counts::execution_counts_for_graph;
use super::profiling::execute_timestamped;
use super::report::{ExecutionReport, ProfiledExecution, RenderGraphProfilingError};
use super::validation::validate_graph;
use super::{compiler, RenderGraphValidationError};

pub struct RenderGraphExecutor {
    context_key: u64,
    pub(super) extension_dispatchers: Arc<ExtensionDispatchRegistry>,
}

impl Default for RenderGraphExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraphExecutor {
    pub fn new() -> Self {
        Self {
            context_key: 0,
            extension_dispatchers: Arc::new(ExtensionDispatchRegistry::new()),
        }
    }

    /// Gán identity ổn định cho device/viewport mà host đang dùng. Hai context
    /// khác nhau không được dùng chung bundle dù logical node giống nhau.
    pub fn with_context_key(context_key: u64) -> Self {
        Self {
            context_key,
            ..Self::new()
        }
    }

    pub fn with_extension_dispatchers(dispatchers: ExtensionDispatchRegistry) -> Self {
        Self {
            context_key: 0,
            extension_dispatchers: Arc::new(dispatchers),
        }
    }

    pub fn with_context_and_extension_dispatchers(
        context_key: u64,
        dispatchers: ExtensionDispatchRegistry,
    ) -> Self {
        Self {
            context_key,
            extension_dispatchers: Arc::new(dispatchers),
        }
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
        validate_graph(
            registry,
            pool,
            graph,
            wgpu::Limits::default().max_bind_groups,
            &self.extension_dispatchers,
        )
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
        validate_graph(
            registry,
            pool,
            graph,
            engine.capabilities().max_bind_groups,
            &self.extension_dispatchers,
        )
    }

    pub fn execute_checked(
        &self,
        engine: &GpuEngine,
        registry: &ResourceRegistry,
        pool: &mut RenderNodePool,
        graph: &RenderGraph,
    ) -> Result<wgpu::SubmissionIndex, RenderGraphValidationError> {
        Ok(self
            .execute_checked_with_report(engine, registry, pool, graph)?
            .submission)
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
        Ok(self
            .execute_with_surface_checked_with_report(engine, registry, pool, graph, surface_view)?
            .submission)
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
        let (
            flattened_nodes,
            draw_commands,
            compute_commands,
            copy_commands,
            indirect_commands,
            declared_usages,
        ) = execution_counts_for_graph(pool, graph)?;
        let submission =
            compiler::execute_unchecked(self, engine, registry, pool, graph, surface_view)?;
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
            engine,
            registry,
            pool,
            graph,
            None,
            profiler,
            resolve_buffer,
            resolve_offset,
            None,
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
            engine,
            registry,
            pool,
            graph,
            None,
            profiler,
            resolve_buffer,
            resolve_offset,
            Some(tracker),
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
            engine,
            registry,
            pool,
            graph,
            surface_view,
            profiler,
            resolve_buffer,
            resolve_offset,
            None,
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
            engine,
            registry,
            pool,
            graph,
            surface_view,
            profiler,
            resolve_buffer,
            resolve_offset,
            Some(tracker),
        )
    }

}
