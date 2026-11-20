use super::PortError;

/// A point in virtual-screen coordinates (physical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenPoint {
    /// Horizontal position.
    pub x: i32,
    /// Vertical position.
    pub y: i32,
}

/// Opaque identifier of a top-level window (an `HWND` on Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowHandle(isize);

impl WindowHandle {
    /// Wraps a raw handle.
    pub const fn new(raw: isize) -> Self {
        Self(raw)
    }

    /// The raw handle.
    pub const fn get(self) -> isize {
        self.0
    }
}

/// The window that was in the foreground before the popup opened: it receives the paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetWindow {
    /// The window.
    pub handle: WindowHandle,
    /// Text caret position, when the application exposes it (HK-005).
    pub caret: Option<ScreenPoint>,
    /// Mouse pointer position when the target was captured (HK-005 fallback).
    pub cursor: ScreenPoint,
    /// The window belongs to an elevated process ClipVault cannot send input to (PST-006).
    pub elevated: bool,
}

/// Tracks and restores the paste target (specs/03 §5.2).
pub trait FocusTracker: Send + Sync {
    /// The current foreground window, unless it belongs to ClipVault itself.
    fn capture_target(&self) -> Option<TargetWindow>;

    /// Brings `target` back to the foreground.
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the window is gone or cannot be activated (PST-005).
    fn restore(&self, target: &TargetWindow) -> Result<(), PortError>;
}

/// Sends synthetic keystrokes to the foreground window.
pub trait KeystrokeSender: Send + Sync {
    /// Sends `Ctrl+V`. Modifiers still held by the user are released first so the target does
    /// not receive e.g. `Ctrl+Shift+V` (SNP-004, PST-002).
    ///
    /// # Errors
    /// [`PortError::Unavailable`] when the input could not be injected.
    fn send_paste(&self) -> Result<(), PortError>;
}
