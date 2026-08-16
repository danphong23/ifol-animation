use thiserror::Error;
use crate::memory::{SubmissionId, SubmissionTracker};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProfilingError {
    #[error("timestamp queries are not supported by this device")]
    UnsupportedTimestampQueries,
    #[error("timestamp writes inside command encoders are not supported by this device")]
    UnsupportedEncoderTimestamps,
    #[error("timestamp query pool must contain at least two slots")]
    InvalidQueryCount,
    #[error("timestamp query pool is exhausted")]
    Exhausted,
    #[error("timestamp query pool is still in flight")]
    InFlight,
    #[error("timestamp span does not belong to this query pool")]
    InvalidSpan,
    #[error("query resolve destination offset must be aligned to {alignment} bytes")]
    MisalignedResolveOffset { alignment: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampSpan {
    first: u32,
    second: u32,
}

impl TimestampSpan {
    pub fn first_query(self) -> u32 { self.first }
    pub fn second_query(self) -> u32 { self.second }
}

/// Primitive profiling tùy chọn. Pool không tự submit queue và không tự map
/// buffer; host quyết định lifecycle theo submission/frame của mình.
#[derive(Debug)]
pub struct TimestampQueryPool {
    query_set: wgpu::QuerySet,
    query_count: u32,
    next_query: u32,
    encoder_timestamps_supported: bool,
    in_flight_until: Option<SubmissionId>,
}

impl TimestampQueryPool {
    pub fn new(device: &wgpu::Device, query_count: u32) -> Result<Self, ProfilingError> {
        if query_count < 2 || !query_count.is_multiple_of(2) {
            return Err(ProfilingError::InvalidQueryCount);
        }
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return Err(ProfilingError::UnsupportedTimestampQueries);
        }
        Ok(Self {
            query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("ifol-gpu-timestamp-query-pool"),
                ty: wgpu::QueryType::Timestamp,
                count: query_count,
            }),
            query_count,
            next_query: 0,
            encoder_timestamps_supported: device.features().contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
            in_flight_until: None,
        })
    }

    pub fn query_set(&self) -> &wgpu::QuerySet { &self.query_set }

    pub fn allocate_span(&mut self) -> Result<TimestampSpan, ProfilingError> {
        if self.in_flight_until.is_some() {
            return Err(ProfilingError::InFlight);
        }
        if self.next_query + 1 >= self.query_count {
            return Err(ProfilingError::Exhausted);
        }
        let span = TimestampSpan { first: self.next_query, second: self.next_query + 1 };
        self.next_query += 2;
        Ok(span)
    }

    /// Gắn pool với submission chứa các query hiện tại.
    pub fn mark_submitted(&mut self, submission: SubmissionId) -> Result<(), ProfilingError> {
        if self.in_flight_until.is_some() {
            return Err(ProfilingError::InFlight);
        }
        self.in_flight_until = Some(submission);
        Ok(())
    }

    /// Reset slot sau khi submission cuối cùng đã hoàn tất. `Ok(false)` nghĩa
    /// là host phải giữ pool và thử lại ở frame sau.
    pub fn reset_after(&mut self, tracker: &SubmissionTracker) -> Result<bool, ProfilingError> {
        if let Some(submission) = self.in_flight_until {
            if !tracker.can_reuse_after(submission) {
                return Ok(false);
            }
        }
        self.next_query = 0;
        self.in_flight_until = None;
        Ok(true)
    }

    pub fn write_span(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        span: TimestampSpan,
    ) -> Result<(), ProfilingError> {
        self.validate_span(span)?;
        if !self.encoder_timestamps_supported {
            return Err(ProfilingError::UnsupportedEncoderTimestamps);
        }
        encoder.write_timestamp(&self.query_set, span.first);
        encoder.write_timestamp(&self.query_set, span.second);
        Ok(())
    }

    pub fn resolve_span(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        span: TimestampSpan,
        destination: &wgpu::Buffer,
        destination_offset: u64,
    ) -> Result<(), ProfilingError> {
        self.validate_span(span)?;
        let alignment = wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT as u64;
        if !destination_offset.is_multiple_of(alignment) {
            return Err(ProfilingError::MisalignedResolveOffset { alignment });
        }
        encoder.resolve_query_set(&self.query_set, span.first..span.second + 1, destination, destination_offset);
        Ok(())
    }

    fn validate_span(&self, span: TimestampSpan) -> Result<(), ProfilingError> {
        if span.first >= self.query_count || span.second != span.first + 1 {
            return Err(ProfilingError::InvalidSpan);
        }
        Ok(())
    }
}

impl Drop for TimestampQueryPool {
    fn drop(&mut self) {
        self.query_set.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::{ProfilingError, TimestampQueryPool};
    use crate::memory::SubmissionTracker;

    #[test]
    fn timestamp_pool_rejects_invalid_shape_without_backend_assumptions() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        assert_eq!(
            TimestampQueryPool::new(engine.device(), 1).unwrap_err(),
            ProfilingError::InvalidQueryCount
        );
        assert_eq!(
            TimestampQueryPool::new(engine.device(), 3).unwrap_err(),
            ProfilingError::InvalidQueryCount
        );
    }

    #[test]
    fn timestamp_pool_reports_optional_backend_support() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        match TimestampQueryPool::new(engine.device(), 4) {
            Err(ProfilingError::UnsupportedTimestampQueries) => {}
            Ok(mut pool) => {
                let first = pool.allocate_span().unwrap();
                assert_eq!((first.first_query(), first.second_query()), (0, 1));
                pool.allocate_span().unwrap();
                assert_eq!(pool.allocate_span(), Err(ProfilingError::Exhausted));
            }
            Err(error) => panic!("unexpected profiling setup error: {error:?}"),
        }
    }

    #[test]
    fn timestamp_pool_reset_waits_for_submission_completion() {
        let engine = pollster::block_on(crate::backend::GpuEngineBuilder::new().build()).unwrap();
        let Ok(mut pool) = TimestampQueryPool::new(engine.device(), 2) else { return; };
        pool.allocate_span().unwrap();
        let mut tracker = SubmissionTracker::new();
        let submission = tracker.begin();
        pool.mark_submitted(submission).unwrap();
        assert_eq!(pool.allocate_span(), Err(ProfilingError::InFlight));
        assert_eq!(pool.reset_after(&tracker), Ok(false));
        tracker.mark_completed(submission);
        assert_eq!(pool.reset_after(&tracker), Ok(true));
        assert_eq!(pool.allocate_span().unwrap().first_query(), 0);
    }
}
