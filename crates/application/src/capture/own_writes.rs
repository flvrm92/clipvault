use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

use crate::ports::ClipboardSequence;

const CAPACITY: usize = 32;

/// Sequence numbers of clipboard changes made by ClipVault itself (CAP-010), shared between
/// the use cases that write to the clipboard and [`super::OwnWriteFilter`].
#[derive(Debug, Default)]
pub struct OwnWriteRegistry {
    pending: Mutex<VecDeque<ClipboardSequence>>,
}

impl OwnWriteRegistry {
    /// Registers a change made by ClipVault. Keeps only the most recent entries.
    pub fn record(&self, sequence: ClipboardSequence) {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        if pending.len() == CAPACITY {
            pending.pop_front();
        }
        pending.push_back(sequence);
    }

    /// Whether `sequence` was registered; consumes the entry.
    pub fn take(&self, sequence: ClipboardSequence) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        pending
            .iter()
            .position(|s| *s == sequence)
            .and_then(|index| pending.remove(index))
            .is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_010_recorded_sequences_are_taken_once() {
        let registry = OwnWriteRegistry::default();
        registry.record(ClipboardSequence::new(1));

        assert!(registry.take(ClipboardSequence::new(1)));
        assert!(!registry.take(ClipboardSequence::new(1)));
        assert!(!registry.take(ClipboardSequence::new(2)));
    }

    #[test]
    fn only_the_most_recent_entries_are_kept() {
        let registry = OwnWriteRegistry::default();
        for raw in 0..=u32::try_from(CAPACITY).unwrap() {
            registry.record(ClipboardSequence::new(raw));
        }

        assert!(
            !registry.take(ClipboardSequence::new(0)),
            "oldest entry evicted"
        );
        assert!(registry.take(ClipboardSequence::new(1)));
    }
}
