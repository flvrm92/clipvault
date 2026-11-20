//! The capture worker thread (specs/03 §4): receives clipboard notifications, debounces them
//! (CAP-012), reads the clipboard and runs [`CaptureClipboard`].

use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use super::Debouncer;
use crate::ports::{ClipboardReader, Clock};
use crate::use_cases::CaptureClipboard;

/// Messages accepted by the worker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerMessage {
    /// The clipboard changed (from the clipboard listener).
    ClipboardChanged,
    /// Finish pending work and stop (TRAY-005).
    Shutdown,
}

/// Debounced clipboard capture loop.
pub struct CaptureWorker {
    reader: Arc<dyn ClipboardReader>,
    capture: CaptureClipboard,
    clock: Arc<dyn Clock>,
    debouncer: Debouncer,
}

impl CaptureWorker {
    /// Builds a worker; `quiet_period` is normally [`Debouncer::DEFAULT_QUIET_PERIOD`].
    pub fn new(
        reader: Arc<dyn ClipboardReader>,
        capture: CaptureClipboard,
        clock: Arc<dyn Clock>,
        quiet_period: Duration,
    ) -> Self {
        Self {
            reader,
            capture,
            clock,
            debouncer: Debouncer::new(quiet_period),
        }
    }

    /// Processes messages until [`WorkerMessage::Shutdown`] or until every sender is dropped.
    /// A change still waiting for its quiet period is captured before returning.
    pub fn run(mut self, inbox: &Receiver<WorkerMessage>) {
        loop {
            let received = match self.debouncer.deadline() {
                None => inbox.recv().map_err(|_| RecvTimeoutError::Disconnected),
                Some(deadline) => inbox.recv_timeout(deadline.elapsed_since(self.clock.now())),
            };
            match received {
                Ok(WorkerMessage::ClipboardChanged) => self.debouncer.notify(self.clock.now()),
                Ok(WorkerMessage::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {}
            }
            if self.debouncer.poll(self.clock.now()) {
                self.capture_now();
            }
        }
        if self.debouncer.deadline().is_some() {
            self.capture_now();
        }
    }

    fn capture_now(&self) {
        let snapshot = match self.reader.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                tracing::warn!(%error, "clipboard could not be read; change skipped");
                return;
            }
        };
        match self.capture.execute(snapshot) {
            Ok(outcome) => tracing::debug!(?outcome, "clipboard change processed"),
            Err(error) => tracing::warn!(%error, "clipboard change could not be stored"),
        }
    }
}
