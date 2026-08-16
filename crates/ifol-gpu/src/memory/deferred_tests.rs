use super::*;

#[test]
fn destruction_waits_for_last_use_completion() {
    let mut queue = DeferredDestructionQueue::new();
    let mut tracker = SubmissionTracker::new();
    let first = tracker.begin();
    let second = tracker.begin();
    queue.defer("first", first);
    queue.defer("second", second);

    assert_eq!(queue.drain_completed(&tracker), Vec::<&str>::new());
    tracker.mark_completed(first);
    assert_eq!(queue.drain_completed(&tracker), vec!["first"]);
    assert_eq!(queue.pending_count(), 1);
    tracker.mark_completed(second);
    assert_eq!(queue.drain_completed(&tracker), vec!["second"]);
}
