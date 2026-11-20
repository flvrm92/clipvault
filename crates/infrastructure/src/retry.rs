//! Retry with backoff for transient failures such as a clipboard held by another process
//! (REL-003: up to 5 retries after 10, 20, 40, 80 and 160 ms).

use std::time::Duration;

/// Delays between attempts; the operation runs `delays.len() + 1` times at most.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    delays: &'static [Duration],
}

impl RetryPolicy {
    /// REL-003 policy for opening the clipboard.
    pub const CLIPBOARD: Self = Self {
        delays: &[
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(40),
            Duration::from_millis(80),
            Duration::from_millis(160),
        ],
    };

    /// A custom policy.
    pub const fn new(delays: &'static [Duration]) -> Self {
        Self { delays }
    }

    /// Runs `attempt` until it succeeds or the delays are exhausted, calling `sleep` between
    /// attempts. Returns the first success or the last error.
    ///
    /// # Errors
    /// The error of the last attempt.
    pub fn run<T, E>(
        &self,
        mut sleep: impl FnMut(Duration),
        mut attempt: impl FnMut() -> Result<T, E>,
    ) -> Result<T, E> {
        let mut result = attempt();
        for delay in self.delays {
            if result.is_ok() {
                break;
            }
            sleep(*delay);
            result = attempt();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(values: &[u64]) -> Vec<Duration> {
        values.iter().copied().map(Duration::from_millis).collect()
    }

    #[test]
    fn rel_003_success_on_first_attempt_does_not_sleep() {
        let mut slept = Vec::new();

        let result: Result<u8, ()> = RetryPolicy::CLIPBOARD.run(|d| slept.push(d), || Ok(7));

        assert_eq!(result, Ok(7));
        assert_eq!(slept, vec![]);
    }

    #[test]
    fn rel_003_retries_with_increasing_delays_until_success() {
        let mut slept = Vec::new();
        let mut calls = 0;

        let result = RetryPolicy::CLIPBOARD.run(
            |d| slept.push(d),
            || {
                calls += 1;
                if calls < 3 { Err(calls) } else { Ok("open") }
            },
        );

        assert_eq!(result, Ok("open"));
        assert_eq!(slept, ms(&[10, 20]));
    }

    #[test]
    fn rel_003_gives_up_after_five_retries_with_the_last_error() {
        let mut slept = Vec::new();
        let mut calls = 0;

        let result: Result<(), u32> = RetryPolicy::CLIPBOARD.run(
            |d| slept.push(d),
            || {
                calls += 1;
                Err(calls)
            },
        );

        assert_eq!(result, Err(6));
        assert_eq!(slept, ms(&[10, 20, 40, 80, 160]));
    }
}
