//! Global keyboard shortcuts (HK-001, HK-003, SNP-006).
//!
//! Pattern: Value Object + Smart Constructor — specs/04 §P-08. A [`Hotkey`] can only be built
//! through validation, so invalid or reserved combinations are unrepresentable.

use std::fmt;
use std::ops::BitOr;
use std::str::FromStr;

use crate::ids::SnippetId;

/// Errors parsing or validating a [`Hotkey`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HotkeyError {
    /// No text was given.
    #[error("shortcut is empty")]
    Empty,
    /// A token is neither a modifier nor a supported key.
    #[error("unknown key `{0}`")]
    UnknownToken(String),
    /// The same modifier appears twice.
    #[error("modifier repeated")]
    DuplicateModifier,
    /// Only modifiers were given.
    #[error("shortcut needs a non-modifier key")]
    MissingKey,
    /// More than one non-modifier key was given.
    #[error("shortcut must have a single non-modifier key")]
    MultipleKeys,
    /// No modifier was given (only F13–F24 may be used alone).
    #[error("shortcut needs Ctrl, Alt, Shift or Win")]
    MissingModifier,
    /// The combination is reserved by Windows or by common editing commands.
    #[error("shortcut is reserved")]
    Reserved,
}

/// A set of modifier keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    /// No modifiers.
    pub const NONE: Self = Self(0);
    /// Control.
    pub const CTRL: Self = Self(1);
    /// Alt.
    pub const ALT: Self = Self(2);
    /// Shift.
    pub const SHIFT: Self = Self(4);
    /// Windows logo key.
    pub const WIN: Self = Self(8);

    /// Canonical display order and names.
    const ORDERED: [(Self, &'static str); 4] = [
        (Self::CTRL, "Ctrl"),
        (Self::ALT, "Alt"),
        (Self::SHIFT, "Shift"),
        (Self::WIN, "Win"),
    ];

    /// Builds a set from its bit representation (unknown bits are ignored).
    pub const fn from_bits(bits: u8) -> Self {
        Self(bits & 0b1111)
    }

    /// Whether every modifier in `other` is present.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Whether the set is empty.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    fn parse_token(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => Some(Self::CTRL),
            "alt" => Some(Self::ALT),
            "shift" => Some(Self::SHIFT),
            "win" | "windows" | "meta" | "super" => Some(Self::WIN),
            _ => None,
        }
    }
}

impl BitOr for Modifiers {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

/// Keys identified by name rather than by a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(missing_docs)] // variant names are the key names
pub enum NamedKey {
    Space,
    Enter,
    Tab,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Up,
    Down,
    Left,
    Right,
    Escape,
    PrintScreen,
    Pause,
    Minus,
    Equal,
    Comma,
    Period,
    Semicolon,
    Slash,
    Backquote,
    BracketLeft,
    BracketRight,
    Backslash,
    Quote,
}

impl NamedKey {
    /// Display name followed by accepted aliases (lowercase).
    const TABLE: [(Self, &'static str, &'static [&'static str]); 28] = [
        (Self::Space, "Space", &[]),
        (Self::Enter, "Enter", &["return"]),
        (Self::Tab, "Tab", &[]),
        (Self::Backspace, "Backspace", &[]),
        (Self::Delete, "Delete", &["del"]),
        (Self::Insert, "Insert", &["ins"]),
        (Self::Home, "Home", &[]),
        (Self::End, "End", &[]),
        (Self::PageUp, "PageUp", &["pgup"]),
        (Self::PageDown, "PageDown", &["pgdn", "pgdown"]),
        (Self::Up, "Up", &["arrowup"]),
        (Self::Down, "Down", &["arrowdown"]),
        (Self::Left, "Left", &["arrowleft"]),
        (Self::Right, "Right", &["arrowright"]),
        (Self::Escape, "Escape", &["esc"]),
        (Self::PrintScreen, "PrintScreen", &["prtsc"]),
        (Self::Pause, "Pause", &[]),
        (Self::Minus, "Minus", &[]),
        (Self::Equal, "Equal", &[]),
        (Self::Comma, "Comma", &[]),
        (Self::Period, "Period", &[]),
        (Self::Semicolon, "Semicolon", &[]),
        (Self::Slash, "Slash", &[]),
        (Self::Backquote, "Backquote", &[]),
        (Self::BracketLeft, "BracketLeft", &[]),
        (Self::BracketRight, "BracketRight", &[]),
        (Self::Backslash, "Backslash", &[]),
        (Self::Quote, "Quote", &[]),
    ];

    fn name(self) -> &'static str {
        Self::TABLE
            .iter()
            .find(|(n, ..)| *n == self)
            .map_or("", |(_, name, _)| name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyCode {
    /// `b'A'..=b'Z'`
    Letter(u8),
    /// `0..=9`
    Digit(u8),
    /// `1..=24`
    Function(u8),
    /// `0..=9`
    Numpad(u8),
    NamedKey(NamedKey),
}

/// What a [`Key`] is, for mapping to platform key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyKind {
    /// `A`–`Z`.
    Letter(char),
    /// `0`–`9` on the main keyboard.
    Digit(u8),
    /// `F1`–`F24`.
    Function(u8),
    /// `0`–`9` on the numeric keypad.
    Numpad(u8),
    /// A named key.
    Named(NamedKey),
}

/// A non-modifier key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(KeyCode);

impl Key {
    /// Every supported key.
    pub fn all() -> impl Iterator<Item = Self> {
        let letters = (b'A'..=b'Z').map(KeyCode::Letter);
        let digits = (0..=9).map(KeyCode::Digit);
        let functions = (1..=24).map(KeyCode::Function);
        let numpad = (0..=9).map(KeyCode::Numpad);
        let named = NamedKey::TABLE
            .into_iter()
            .map(|(n, ..)| KeyCode::NamedKey(n));
        letters
            .chain(digits)
            .chain(functions)
            .chain(numpad)
            .chain(named)
            .map(Self)
    }

    /// The key's kind.
    pub fn kind(self) -> KeyKind {
        match self.0 {
            KeyCode::Letter(c) => KeyKind::Letter(char::from(c)),
            KeyCode::Digit(d) => KeyKind::Digit(d),
            KeyCode::Function(n) => KeyKind::Function(n),
            KeyCode::Numpad(d) => KeyKind::Numpad(d),
            KeyCode::NamedKey(n) => KeyKind::Named(n),
        }
    }

    const fn letter(c: u8) -> Self {
        Self(KeyCode::Letter(c))
    }

    /// F13–F24 are unused by applications and may be bound without modifiers.
    fn usable_alone(self) -> bool {
        matches!(self.0, KeyCode::Function(13..=24))
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            KeyCode::Letter(c) => write!(f, "{}", char::from(c)),
            KeyCode::Digit(d) => write!(f, "{d}"),
            KeyCode::Function(n) => write!(f, "F{n}"),
            KeyCode::Numpad(d) => write!(f, "Num{d}"),
            KeyCode::NamedKey(n) => f.write_str(n.name()),
        }
    }
}

impl FromStr for Key {
    type Err = HotkeyError;

    fn from_str(token: &str) -> Result<Self, Self::Err> {
        let lower = token.to_ascii_lowercase();
        let unknown = || HotkeyError::UnknownToken(token.to_owned());
        let number = |rest: &str| rest.parse::<u8>().ok();

        let code = match lower.as_bytes() {
            [c @ b'a'..=b'z'] => KeyCode::Letter(c.to_ascii_uppercase()),
            [d @ b'0'..=b'9'] => KeyCode::Digit(d - b'0'),
            _ => {
                if let Some(n) = lower
                    .strip_prefix('f')
                    .and_then(number)
                    .filter(|n| (1..=24).contains(n))
                {
                    KeyCode::Function(n)
                } else if let Some(d) = lower
                    .strip_prefix("num")
                    .and_then(number)
                    .filter(|d| *d <= 9)
                {
                    KeyCode::Numpad(d)
                } else {
                    let (named, ..) = NamedKey::TABLE
                        .iter()
                        .find(|(_, name, aliases)| {
                            name.eq_ignore_ascii_case(&lower) || aliases.contains(&lower.as_str())
                        })
                        .ok_or_else(unknown)?;
                    KeyCode::NamedKey(*named)
                }
            }
        };
        Ok(Self(code))
    }
}

/// Combinations that cannot be bound (HK-003).
const RESERVED: [(Modifiers, Key); 9] = [
    (Modifiers::CTRL, Key::letter(b'C')),
    (Modifiers::CTRL, Key::letter(b'V')),
    (Modifiers::CTRL, Key::letter(b'X')),
    (Modifiers::CTRL, Key::letter(b'Z')),
    (Modifiers::WIN, Key::letter(b'L')),
    (Modifiers::WIN, Key::letter(b'V')),
    (
        Modifiers(Modifiers::CTRL.0 | Modifiers::ALT.0),
        Key(KeyCode::NamedKey(NamedKey::Delete)),
    ),
    (Modifiers::ALT, Key(KeyCode::Function(4))),
    (Modifiers::ALT, Key(KeyCode::NamedKey(NamedKey::Tab))),
];

/// A valid, non-reserved global shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hotkey {
    modifiers: Modifiers,
    key: Key,
}

impl Hotkey {
    /// Validates a combination of modifiers and key.
    ///
    /// # Errors
    /// [`HotkeyError::MissingModifier`] or [`HotkeyError::Reserved`].
    pub fn new(modifiers: Modifiers, key: Key) -> Result<Self, HotkeyError> {
        if modifiers.is_empty() && !key.usable_alone() {
            return Err(HotkeyError::MissingModifier);
        }
        if RESERVED.contains(&(modifiers, key)) {
            return Err(HotkeyError::Reserved);
        }
        Ok(Self { modifiers, key })
    }

    /// Parses text such as `"Ctrl+Shift+1"`: case-insensitive, any modifier order, spaces allowed.
    ///
    /// # Errors
    /// Any [`HotkeyError`] describing why the text is not a valid shortcut.
    pub fn parse(text: &str) -> Result<Self, HotkeyError> {
        if text.trim().is_empty() {
            return Err(HotkeyError::Empty);
        }
        let mut modifiers = Modifiers::NONE;
        let mut key = None;
        for token in text.split('+').map(str::trim) {
            if let Some(modifier) = Modifiers::parse_token(token) {
                if modifiers.contains(modifier) {
                    return Err(HotkeyError::DuplicateModifier);
                }
                modifiers = modifiers | modifier;
            } else if key.replace(token.parse::<Key>()?).is_some() {
                return Err(HotkeyError::MultipleKeys);
            }
        }
        Self::new(modifiers, key.ok_or(HotkeyError::MissingKey)?)
    }

    /// The default shortcut that opens the history popup: `Ctrl+Win+V` (HK-001).
    pub const fn default_open() -> Self {
        Self {
            modifiers: Modifiers(Modifiers::CTRL.0 | Modifiers::WIN.0),
            key: Key::letter(b'V'),
        }
    }

    /// Modifier keys.
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Non-modifier key.
    pub const fn key(&self) -> Key {
        self.key
    }
}

impl fmt::Display for Hotkey {
    /// Canonical form: `Ctrl+Alt+Shift+Win+Key`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (modifier, name) in Modifiers::ORDERED {
            if self.modifiers.contains(modifier) {
                write!(f, "{name}+")?;
            }
        }
        write!(f, "{}", self.key)
    }
}

impl FromStr for Hotkey {
    type Err = HotkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// What a ClipVault shortcut is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyOwner {
    /// Opens the history popup.
    OpenHistory,
    /// Pastes a snippet.
    Snippet(SnippetId),
}

/// Finds which other binding already uses `candidate` (HK-003). The binding being edited
/// (`editing`) is ignored so that saving it unchanged is not a conflict.
pub fn find_conflict<'a>(
    candidate: &Hotkey,
    bindings: impl IntoIterator<Item = (HotkeyOwner, &'a Hotkey)>,
    editing: HotkeyOwner,
) -> Option<HotkeyOwner> {
    bindings
        .into_iter()
        .find(|(owner, hotkey)| *owner != editing && *hotkey == candidate)
        .map(|(owner, _)| owner)
}

#[cfg(test)]
mod tests;
