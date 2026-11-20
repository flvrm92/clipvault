//! [`Clock`] backed by the system time, and a real [`Delay`].

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use clipvault_application::ports::{Clock, Delay};
use clipvault_domain::time::Timestamp;

/// The wall clock (UTC).
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
        Timestamp::from_millis(millis)
    }
}

/// Sleeps the calling thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThreadDelay;

impl Delay for ThreadDelay {
    fn wait(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_is_after_2026_and_monotonic_enough() {
        let first = SystemClock.now();
        let second = SystemClock.now();

        assert!(first.as_millis() > 1_767_225_600_000, "after 2026-01-01");
        assert!(second >= first);
    }

    #[test]
    fn thread_delay_waits_at_least_the_requested_time() {
        let start = std::time::Instant::now();

        ThreadDelay.wait(Duration::from_millis(20));

        assert!(start.elapsed() >= Duration::from_millis(20));
    }
}
