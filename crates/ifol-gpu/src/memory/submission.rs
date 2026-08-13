/// Identity logic của một submission, tách khỏi kiểu backend cụ thể của `wgpu`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SubmissionId(pub u64);

#[derive(Debug, Default)]
pub struct SubmissionTracker {
    next: u64,
    completed: SubmissionId,
}

impl SubmissionTracker {
    pub fn new() -> Self { Self::default() }

    pub fn begin(&mut self) -> SubmissionId {
        self.next = self.next.saturating_add(1);
        SubmissionId(self.next)
    }

    pub fn mark_completed(&mut self, submission: SubmissionId) {
        if submission > self.completed { self.completed = submission; }
    }

    pub fn completed(&self) -> SubmissionId { self.completed }

    pub fn is_completed(&self, submission: SubmissionId) -> bool { submission <= self.completed }

    pub fn can_reuse_after(&self, submission: SubmissionId) -> bool { self.is_completed(submission) }
}

#[cfg(test)]
mod tests {
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
}
