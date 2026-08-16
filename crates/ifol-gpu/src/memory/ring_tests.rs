use super::*;
use crate::backend::GpuEngineBuilder;

#[test]
fn test_ring_buffer_wrap_around() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    // Mock thông số căn lề chuẩn là 256 bytes để dễ test logic
    let alignment = 256;

    // Cố tình tạo buffer nhỏ (1024 bytes) để test wrap around dễ dàng
    let mut ring = UniformRingBuffer::new(engine.device(), 1024, alignment);

    // Cấp phát 100 bytes -> bị ép căn lề thành 256 bytes.
    assert_eq!(ring.allocate(100), Some(0));
    assert_eq!(ring.allocate(200), Some(256));
    assert_eq!(ring.allocate(500), Some(512));
    // 512 + 512 (căn lề) = 1024. Đã xài hết buffer.

    // Không được tự wrap và ghi đè allocation cũ.
    assert_eq!(ring.allocate(100), None);
    assert_eq!(ring.current_offset, 1024);

    ring.reset();
    assert_eq!(ring.allocate(100), Some(0));
}

#[test]
fn ring_rejects_zero_and_overflowing_requests() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut ring = UniformRingBuffer::new(engine.device(), 1024, 256);

    assert_eq!(ring.allocate(0), None);
    assert_eq!(ring.allocate(u64::MAX), None);
}

#[test]
fn reset_waits_for_submission_completion() {
    let engine = pollster::block_on(GpuEngineBuilder::new().build()).unwrap();
    let mut ring = UniformRingBuffer::new(engine.device(), 1024, 256);
    assert_eq!(ring.allocate(128), Some(0));
    let mut tracker = SubmissionTracker::new();
    let submission = tracker.begin();

    assert!(!ring.reset_after(&tracker, submission));
    assert_eq!(ring.allocate(128), Some(256));

    tracker.mark_completed(submission);
    assert!(ring.reset_after(&tracker, submission));
    assert_eq!(ring.allocate(128), Some(0));
}
