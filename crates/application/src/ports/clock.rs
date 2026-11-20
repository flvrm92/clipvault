use clipvault_domain::time::Timestamp;

/// Source of the current time; injectable for deterministic tests.
pub trait Clock: Send + Sync {
    /// The current instant.
    fn now(&self) -> Timestamp;
}
