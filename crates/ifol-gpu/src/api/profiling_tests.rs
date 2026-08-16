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
    let Ok(mut pool) = TimestampQueryPool::new(engine.device(), 2) else {
        return;
    };
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
