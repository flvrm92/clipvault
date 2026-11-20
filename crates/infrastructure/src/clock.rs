//! [`Clock`] backed by the system time.

use std::time::{SystemTime, UNIX_EPOCH};

use clipvault_application::ports::Clock;
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
}
