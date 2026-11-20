//! Points in time as Unix epoch milliseconds (UTC), as stored in the database (specs/05 §1).

use std::ops::Add;
use std::time::Duration;

/// A UTC instant with millisecond precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(i64);

impl Timestamp {
    /// Builds a timestamp from Unix epoch milliseconds.
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// Unix epoch milliseconds.
    pub const fn as_millis(self) -> i64 {
        self.0
    }

    /// Time elapsed from `earlier` to `self`; zero if `earlier` is in the future.
    pub fn elapsed_since(self, earlier: Self) -> Duration {
        u64::try_from(self.0.saturating_sub(earlier.0))
            .map_or(Duration::ZERO, Duration::from_millis)
    }
}

impl Add<Duration> for Timestamp {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        let millis = i64::try_from(rhs.as_millis()).unwrap_or(i64::MAX);
        Self(self.0.saturating_add(millis))
    }
}

#[cfg(test)]
mod tests;
