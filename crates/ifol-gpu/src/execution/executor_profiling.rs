use crate::api::TimestampQueryPool;
use crate::backend::GpuEngine;
use crate::graph::{RenderGraph, RenderNodePool};
use crate::memory::SubmissionTracker;
use crate::resources::ResourceRegistry;

use super::profiling::execute_timestamped;
use super::{ProfiledExecution, RenderGraphExecutor, RenderGraphProfilingError};

impl RenderGraphExecutor {
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
