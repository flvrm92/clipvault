//! [`KeystrokeSender`] with `SendInput` (PST-001, SNP-004).

use std::time::{Duration, Instant};

use clipvault_application::ports::{KeystrokeSender, PortError};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN,
    VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_V,
};

/// How long to wait for the user to release modifiers before releasing them synthetically.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_secs(1);

/// Modifiers that would change `Ctrl+V` into another shortcut if still held. The generic codes
/// are checked too: input injected by other tools often uses them instead of the left/right ones.
const MODIFIERS: [VIRTUAL_KEY; 11] = [
    VK_SHIFT,
    VK_LSHIFT,
    VK_RSHIFT,
    VK_CONTROL,
    VK_LCONTROL,
    VK_RCONTROL,
    VK_MENU,
    VK_LMENU,
    VK_RMENU,
    VK_LWIN,
    VK_RWIN,
];

/// An unassigned virtual key: pressing it before releasing Win keeps the Start menu closed.
const VK_NEUTRAL: VIRTUAL_KEY = VIRTUAL_KEY(0xE8);

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                ..Default::default()
            },
        },
    }
}

fn is_down(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: no preconditions; the high bit reports the physical state.
    (unsafe { GetAsyncKeyState(i32::from(vk.0)) }).cast_unsigned() & 0x8000 != 0
}

fn held_modifiers() -> Vec<VIRTUAL_KEY> {
    MODIFIERS.into_iter().filter(|vk| is_down(*vk)).collect()
}

fn send(inputs: &[INPUT]) -> Result<(), PortError> {
    let size = i32::try_from(size_of::<INPUT>()).unwrap_or(i32::MAX);
    // SAFETY: `inputs` is a valid slice of initialized keyboard inputs of the given size.
    let sent = unsafe { SendInput(inputs, size) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err(PortError::Unavailable("keyboard input was blocked".into()))
    }
}

/// Waits for the user to release modifiers; releases any still held after the timeout.
fn release_modifiers() -> Result<(), PortError> {
    let deadline = Instant::now() + MODIFIER_RELEASE_TIMEOUT;
    let mut held = held_modifiers();
    while !held.is_empty() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        held = held_modifiers();
    }
    if held.is_empty() {
        return Ok(());
    }
    let mut inputs = Vec::new();
    if held.iter().any(|vk| [VK_LWIN, VK_RWIN].contains(vk)) {
        inputs.extend([key(VK_NEUTRAL, false), key(VK_NEUTRAL, true)]);
    }
    inputs.extend(held.into_iter().map(|vk| key(vk, true)));
    send(&inputs)
}

/// Sends keystrokes to the foreground window.
#[derive(Debug, Clone, Copy, Default)]
pub struct Win32KeystrokeSender;

impl KeystrokeSender for Win32KeystrokeSender {
    fn send_paste(&self) -> Result<(), PortError> {
        release_modifiers()?;
        send(&[
            key(VK_CONTROL, false),
            key(VK_V, false),
            key(VK_V, true),
            key(VK_CONTROL, true),
        ])
    }
}
