use crate::api::{ProfilingError, TimestampSpan};
use crate::memory::SubmissionId;

use super::RenderGraphValidationError;
use thiserror::Error;

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
