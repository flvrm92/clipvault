/// Notifications for the UI (pattern: Observer — specs/04 §P-07). Events say *what* changed,
/// so subscribers reload; they do not carry incremental state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppEvent {
    /// History items were added, changed or removed.
    HistoryChanged,
    /// Capture was paused or resumed (CAP-008, TRAY-003).
    CaptureStateChanged,
}

/// Publishes [`AppEvent`]s.
pub trait EventPublisher: Send + Sync {
    /// Publishes an event; delivery failures are the adapter's concern.
    fn publish(&self, event: AppEvent);
}
