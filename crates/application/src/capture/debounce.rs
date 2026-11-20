use std::time::Duration;

use clipvault_domain::time::Timestamp;

/// Collapses bursts of clipboard notifications into one read (CAP-012): fires once the
/// notifications have been quiet for the configured period.
#[derive(Debug, Clone)]
pub struct Debouncer {
    quiet_period: Duration,
    last_event: Option<Timestamp>,
}

impl Debouncer {
    /// Quiet period of specs/01 CAP-012.
    pub const DEFAULT_QUIET_PERIOD: Duration = Duration::from_millis(100);

    /// A debouncer with no pending notification.
    pub const fn new(quiet_period: Duration) -> Self {
        Self {
            quiet_period,
            last_event: None,
        }
    }

    /// Records a notification.
    pub const fn notify(&mut self, at: Timestamp) {
        self.last_event = Some(at);
    }

    /// When the pending notification becomes ready, if any (for timer scheduling).
    pub fn deadline(&self) -> Option<Timestamp> {
        self.last_event.map(|at| at + self.quiet_period)
    }

    /// Returns `true` exactly once per burst, when `now` is past the quiet period.
    pub fn poll(&mut self, now: Timestamp) -> bool {
        let ready = self.deadline().is_some_and(|deadline| now >= deadline);
        if ready {
            self.last_event = None;
        }
        ready
    }
}

impl Default for Debouncer {
    fn default() -> Self {
        Self::new(Self::DEFAULT_QUIET_PERIOD)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: i64) -> Timestamp {
        Timestamp::from_millis(ms)
    }

    #[test]
    fn cap_012_single_notification_fires_after_the_quiet_period() {
        let mut debouncer = Debouncer::default();
        debouncer.notify(at(1_000));

        assert!(!debouncer.poll(at(1_099)));
        assert!(debouncer.poll(at(1_100)));
    }

    #[test]
    fn cap_012_a_burst_fires_once_after_the_last_notification() {
        let mut debouncer = Debouncer::default();
        for ms in [1_000, 1_030, 1_060, 1_090] {
            debouncer.notify(at(ms));
            assert!(!debouncer.poll(at(ms + 5)));
        }

        assert_eq!(debouncer.deadline(), Some(at(1_190)));
        assert!(!debouncer.poll(at(1_150)));
        assert!(debouncer.poll(at(1_190)));
        assert!(!debouncer.poll(at(2_000)), "fires only once");
    }

    #[test]
    fn cap_012_nothing_fires_without_notifications() {
        let mut debouncer = Debouncer::new(Duration::from_millis(10));

        assert_eq!(debouncer.deadline(), None);
        assert!(!debouncer.poll(at(1_000_000)));
    }
}
