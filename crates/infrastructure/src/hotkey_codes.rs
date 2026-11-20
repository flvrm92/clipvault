//! [`Hotkey`] → `RegisterHotKey` arguments (modifier flags and virtual-key code). Pure code with
//! the Win32 constant values, so it is tested on any platform.

use clipvault_domain::hotkey::{Hotkey, KeyKind, Modifiers, NamedKey};

/// `MOD_ALT`.
pub const MOD_ALT: u32 = 0x0001;
/// `MOD_CONTROL`.
pub const MOD_CONTROL: u32 = 0x0002;
/// `MOD_SHIFT`.
pub const MOD_SHIFT: u32 = 0x0004;
/// `MOD_WIN`.
pub const MOD_WIN: u32 = 0x0008;
/// `MOD_NOREPEAT`: holding the keys does not fire the shortcut repeatedly.
pub const MOD_NOREPEAT: u32 = 0x4000;

const fn named_vk(key: NamedKey) -> u32 {
    match key {
        NamedKey::Space => 0x20,
        NamedKey::Enter => 0x0D,
        NamedKey::Tab => 0x09,
        NamedKey::Backspace => 0x08,
        NamedKey::Delete => 0x2E,
        NamedKey::Insert => 0x2D,
        NamedKey::Home => 0x24,
        NamedKey::End => 0x23,
        NamedKey::PageUp => 0x21,
        NamedKey::PageDown => 0x22,
        NamedKey::Up => 0x26,
        NamedKey::Down => 0x28,
        NamedKey::Left => 0x25,
        NamedKey::Right => 0x27,
        NamedKey::Escape => 0x1B,
        NamedKey::PrintScreen => 0x2C,
        NamedKey::Pause => 0x13,
        NamedKey::Minus => 0xBD,        // VK_OEM_MINUS
        NamedKey::Equal => 0xBB,        // VK_OEM_PLUS
        NamedKey::Comma => 0xBC,        // VK_OEM_COMMA
        NamedKey::Period => 0xBE,       // VK_OEM_PERIOD
        NamedKey::Semicolon => 0xBA,    // VK_OEM_1
        NamedKey::Slash => 0xBF,        // VK_OEM_2
        NamedKey::Backquote => 0xC0,    // VK_OEM_3
        NamedKey::BracketLeft => 0xDB,  // VK_OEM_4
        NamedKey::Backslash => 0xDC,    // VK_OEM_5
        NamedKey::BracketRight => 0xDD, // VK_OEM_6
        NamedKey::Quote => 0xDE,        // VK_OEM_7
    }
}

/// Virtual-key code of the hotkey's key.
pub fn virtual_key(hotkey: &Hotkey) -> u32 {
    match hotkey.key().kind() {
        KeyKind::Letter(c) => u32::from(c),
        KeyKind::Digit(d) => 0x30 + u32::from(d),
        KeyKind::Function(n) => 0x70 + u32::from(n) - 1,
        KeyKind::Numpad(d) => 0x60 + u32::from(d),
        KeyKind::Named(named) => named_vk(named),
    }
}

/// `RegisterHotKey` modifier flags, always with `MOD_NOREPEAT`.
pub fn modifier_flags(hotkey: &Hotkey) -> u32 {
    [
        (Modifiers::ALT, MOD_ALT),
        (Modifiers::CTRL, MOD_CONTROL),
        (Modifiers::SHIFT, MOD_SHIFT),
        (Modifiers::WIN, MOD_WIN),
    ]
    .into_iter()
    .filter(|(modifier, _)| hotkey.modifiers().contains(*modifier))
    .fold(MOD_NOREPEAT, |flags, (_, flag)| flags | flag)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use clipvault_domain::hotkey::Key;

    use super::*;

    fn hk(text: &str) -> Hotkey {
        Hotkey::parse(text).unwrap()
    }

    #[test]
    fn hk_001_maps_modifiers_and_keys_to_win32_codes() {
        assert_eq!(virtual_key(&hk("Ctrl+Win+V")), 0x56);
        assert_eq!(
            modifier_flags(&hk("Ctrl+Win+V")),
            MOD_CONTROL | MOD_WIN | MOD_NOREPEAT
        );
        assert_eq!(virtual_key(&hk("Ctrl+Shift+1")), 0x31);
        assert_eq!(
            modifier_flags(&hk("Ctrl+Shift+1")),
            MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT
        );
        assert_eq!(virtual_key(&hk("F13")), 0x7C);
        assert_eq!(modifier_flags(&hk("F13")), MOD_NOREPEAT);
        assert_eq!(virtual_key(&hk("Alt+F24")), 0x87);
        assert_eq!(virtual_key(&hk("Ctrl+Num0")), 0x60);
        assert_eq!(virtual_key(&hk("Ctrl+Quote")), 0xDE);
    }

    #[test]
    fn every_supported_key_has_a_distinct_virtual_key() {
        let codes: HashSet<u32> = Key::all()
            .filter_map(|key| Hotkey::new(Modifiers::CTRL | Modifiers::SHIFT, key).ok())
            .map(|hotkey| virtual_key(&hotkey))
            .collect();

        assert_eq!(codes.len(), Key::all().count());
        assert!(codes.iter().all(|code| (0x08..=0xFE).contains(code)));
    }
}
