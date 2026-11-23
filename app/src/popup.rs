//! The history popup window: pre-created hidden (PERF-001), shown next to the caret or pointer
//! (HK-005), hidden on blur (PST-007).

use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use clipvault_application::placement::{ScreenRect, anchor_of, place_popup};
use clipvault_application::ports::{FocusTracker, ScreenPoint, TargetWindow};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, Wry};

use crate::container::Container;

/// Window label in `tauri.conf.json`.
pub const POPUP: &str = "popup";

/// A tray click right after the popup was hidden by losing focus must not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

/// Paste target and visibility bookkeeping.
#[derive(Default)]
pub struct PopupState {
    target: Mutex<Option<TargetWindow>>,
    hidden_at: Mutex<Option<Instant>>,
}

impl PopupState {
    /// Takes the window captured when the popup opened.
    pub fn take_target(&self) -> Option<TargetWindow> {
        self.target
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Captures the paste target, positions the popup and shows it.
pub fn show(app: &AppHandle<Wry>) {
    let Some(window) = app.get_webview_window(POPUP) else {
        return;
    };
    let target = app.state::<Container>().focus.capture_target();
    *app.state::<PopupState>()
        .target
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = target;

    let pointer = app.cursor_position().map_or(ScreenPoint::default(), |p| {
        // Screen coordinates are whole pixels well within i32.
        #[allow(clippy::cast_possible_truncation)]
        ScreenPoint {
            x: p.x.round() as i32,
            y: p.y.round() as i32,
        }
    });
    let anchor = anchor_of(target.as_ref(), pointer);
    let monitor = app
        .monitor_from_point(f64::from(anchor.x), f64::from(anchor.y))
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    if let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) {
        let area = monitor.work_area();
        let work_area = ScreenRect {
            x: area.position.x,
            y: area.position.y,
            width: area.size.width,
            height: area.size.height,
        };
        let position = place_popup(anchor, size.width, size.height, work_area);
        let _ = window.set_position(PhysicalPosition::new(position.x, position.y));
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to(POPUP, "popup://shown", ());
}

/// Hides the popup.
pub fn hide(app: &AppHandle<Wry>) {
    if let Some(window) = app.get_webview_window(POPUP)
        && window.is_visible().unwrap_or(false)
    {
        let _ = window.hide();
        *app.state::<PopupState>()
            .hidden_at
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Instant::now());
        let _ = app.emit_to(POPUP, "popup://hidden", ());
    }
}

/// Opens or closes the popup (tray click, TRAY-002).
pub fn toggle(app: &AppHandle<Wry>) {
    let visible = app
        .get_webview_window(POPUP)
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    if visible {
        hide(app);
        return;
    }
    let recently_hidden = app
        .state::<PopupState>()
        .hidden_at
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
    if !recently_hidden {
        show(app);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    /// Permissions each capability file grants, keyed by window label.
    fn grants(capability: &str, window: &str, permission: &str) -> bool {
        let Ok(json) = serde_json::from_str::<Value>(capability) else {
            panic!("capability file is not valid JSON");
        };
        let listed = |key: &str, wanted: &str| {
            json[key]
                .as_array()
                .is_some_and(|values| values.iter().any(|v| v.as_str() == Some(wanted)))
        };
        listed("windows", window) && listed("permissions", permission)
    }

    const CAPABILITIES: [&str; 2] = [
        include_str!("../capabilities/default.json"),
        include_str!("../capabilities/popup.json"),
    ];

    fn any_grants(window: &str, permission: &str) -> bool {
        CAPABILITIES.iter().any(|c| grants(c, window, permission))
    }

    #[test]
    fn hk_006_only_the_popup_may_be_dragged() {
        const DRAG: &str = "core:window:allow-start-dragging";
        assert!(any_grants(super::POPUP, DRAG));
        assert!(!any_grants("settings", DRAG));
        assert!(!any_grants("welcome", DRAG));
    }

    #[test]
    fn hk_006_double_clicking_the_header_does_not_maximize_the_popup() {
        assert!(!any_grants(
            super::POPUP,
            "core:window:allow-internal-toggle-maximize"
        ));
    }
}
