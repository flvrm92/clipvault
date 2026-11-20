//! The Windows accent color palette (THM-004).

use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_BINARY, RegGetValueW};
use windows::core::{PCWSTR, w};

const ACCENT: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Accent");

/// Eight RGBA entries: light 3, light 2, light 1, base, dark 1, dark 2, dark 3, unused.
const PALETTE_BYTES: usize = 32;
const LIGHT_2: usize = 1;
const DARK_1: usize = 4;

/// An opaque sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
}

impl Rgb {
    /// `#rrggbb`.
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// The accent shades Windows itself uses on each theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentColors {
    /// For light surfaces (palette "dark 1").
    pub on_light: Rgb,
    /// For dark surfaces (palette "light 2").
    pub on_dark: Rgb,
}

/// Parses the `AccentPalette` registry value.
pub fn parse_palette(bytes: &[u8]) -> Option<AccentColors> {
    if bytes.len() < PALETTE_BYTES {
        return None;
    }
    let entry = |index: usize| {
        let rgba = bytes.get(index * 4..index * 4 + 3)?;
        Some(Rgb {
            r: rgba[0],
            g: rgba[1],
            b: rgba[2],
        })
    };
    Some(AccentColors {
        on_light: entry(DARK_1)?,
        on_dark: entry(LIGHT_2)?,
    })
}

/// The current accent palette; `None` when Windows does not expose one.
pub fn system_accent() -> Option<AccentColors> {
    let mut buffer = [0_u8; PALETTE_BYTES];
    let mut size = u32::try_from(buffer.len()).ok()?;
    // SAFETY: `buffer` and `size` describe a writable byte buffer.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            ACCENT,
            w!("AccentPalette"),
            RRF_RT_REG_BINARY,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&raw mut size),
        )
    };
    if status.is_err() {
        return None;
    }
    parse_palette(buffer.get(..size as usize)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thm_004_palette_entries_map_to_the_theme_shades() {
        let mut bytes = [0_u8; 32];
        bytes[4..8].copy_from_slice(&[0xa6, 0xd8, 0xff, 0x00]); // light 2
        bytes[16..20].copy_from_slice(&[0x00, 0x5a, 0x9e, 0x00]); // dark 1

        let colors = parse_palette(&bytes).unwrap();

        assert_eq!(colors.on_dark.to_hex(), "#a6d8ff");
        assert_eq!(colors.on_light.to_hex(), "#005a9e");
    }

    #[test]
    fn thm_004_short_values_are_ignored() {
        assert_eq!(parse_palette(&[0; 31]), None);
    }

    #[test]
    fn the_running_system_can_be_queried() {
        let _ = system_accent();
    }
}
