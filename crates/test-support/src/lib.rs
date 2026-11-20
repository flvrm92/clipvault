//! Shared test doubles and builders (fakes, test data builders).
//!
//! Only to be used as a `dev-dependency`. See `specs/04-padroes-de-projeto.md` P-10 and P-11:
//! hand-written, stateful fakes are preferred over mocks.
#![forbid(unsafe_code)]
// Test-only crate: failing fast on invalid fixtures is the desired behaviour (REL-004 covers
// production code only).
#![allow(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc
)]

pub mod contracts;
mod fakes;
pub mod snapshot;

pub use fakes::{
    FakeClipboardReader, FakeFocusTracker, FakeHotkeyRegistrar, FakeRunningApps, FixedClock,
    FixedSettings, InMemoryHistoryRepository, InMemoryImageStore, InMemorySnippetRepository,
    RealClock, RecordingClipboardWriter, RecordingDelay, RecordingEvents, RecordingKeystrokes,
};

use std::sync::{Mutex, MutexGuard, PoisonError};

/// Locks a mutex, ignoring poisoning (a panicking test already failed).
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
