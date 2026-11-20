//! Shared test doubles and builders (fakes, test data builders).
//!
//! Only to be used as a `dev-dependency`. See `specs/04-padroes-de-projeto.md` P-10 and P-11:
//! hand-written, stateful fakes are preferred over mocks.
#![forbid(unsafe_code)]
// Test-only crate: failing fast on invalid fixtures is the desired behaviour (REL-004 covers
// production code only).
#![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

mod fakes;
pub mod snapshot;

pub use fakes::{
    FixedClock, FixedSettings, InMemoryHistoryRepository, InMemoryImageStore, RecordingEvents,
};

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks a mutex, ignoring poisoning (a panicking test already failed).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
