use super::*;

#[test]
fn submissions_are_monotonic_and_reuse_waits_for_completion() {
    let mut tracker = SubmissionTracker::new();
    let first = tracker.begin();
    let second = tracker.begin();
    assert!(first < second);
    assert!(!tracker.can_reuse_after(first));
    tracker.mark_completed(second);
    assert!(tracker.can_reuse_after(first));
    assert!(tracker.can_reuse_after(second));
}

#[test]
fn late_completion_notification_cannot_move_tracker_backwards() {
    let mut tracker = SubmissionTracker::new();
    let first = tracker.begin();
    let second = tracker.begin();
    tracker.mark_completed(second);
    tracker.mark_completed(first);
    assert_eq!(tracker.completed(), second);
}
