//! Effective theme and window backdrop (THM-001..003).

use clipvault_domain::settings::{ThemeMode, UiSettings};
use clipvault_infrastructure::win32::{
    AccentColors, SystemAppearance, system_accent, system_appearance,
};
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect;
use tauri::{WebviewWindow, Wry};

use crate::dto::{AppearanceDto, BackdropDto, ThemeDto};

/// Resolves the user preferences against the system settings (THM-001/002/004).
pub fn resolve(
    ui: UiSettings,
    system: SystemAppearance,
    accent: Option<AccentColors>,
) -> AppearanceDto {
    let mode = ui.theme;
    let theme = match (mode, system.apps_light) {
        (ThemeMode::Light, _) | (ThemeMode::System, true) => ThemeDto::Light,
        (ThemeMode::Dark, _) | (ThemeMode::System, false) => ThemeDto::Dark,
    };
    let backdrop = if system.windows_11 && system.transparency {
        BackdropDto::Acrylic
    } else {
        BackdropDto::Solid
    };
    let accent = accent.filter(|_| ui.use_accent_color).map(|colors| {
        match theme {
            ThemeDto::Light => colors.on_light,
            ThemeDto::Dark => colors.on_dark,
        }
        .to_hex()
    });
    AppearanceDto {
        theme,
        backdrop,
        accent,
    }
}

/// The appearance for the current settings.
pub fn current(ui: UiSettings) -> AppearanceDto {
    let accent = if ui.use_accent_color {
        system_accent()
    } else {
        None
    };
    resolve(ui, system_appearance(), accent)
}

/// Applies the backdrop to a window created with `transparent: true` (THM-002).
pub fn apply_backdrop(window: &WebviewWindow<Wry>, appearance: &AppearanceDto) {
    let effects = match appearance.backdrop {
        BackdropDto::Acrylic => Some(WindowEffectsConfig {
            effects: vec![Effect::Acrylic],
            ..WindowEffectsConfig::default()
        }),
        BackdropDto::Solid => None,
    };
    if let Err(error) = window.set_effects(effects) {
        tracing::warn!(%error, "window backdrop could not be applied");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use clipvault_infrastructure::win32::Rgb;

    fn ui(theme: ThemeMode) -> UiSettings {
        UiSettings {
            theme,
            ..UiSettings::default()
        }
    }

    const ACCENT: AccentColors = AccentColors {
        on_light: Rgb {
            r: 0,
            g: 0x5a,
            b: 0x9e,
        },
        on_dark: Rgb {
            r: 0xa6,
            g: 0xd8,
            b: 0xff,
        },
    };

    #[test]
    fn thm_004_accent_is_sent_only_when_enabled_and_matches_the_theme() {
        let enabled = |theme| UiSettings {
            use_accent_color: true,
            ..ui(theme)
        };
        let light = SystemAppearance {
            apps_light: true,
            ..WIN11_DARK
        };

        assert_eq!(
            resolve(ui(ThemeMode::Dark), WIN11_DARK, Some(ACCENT)).accent,
            None
        );
        assert_eq!(
            resolve(enabled(ThemeMode::Dark), WIN11_DARK, Some(ACCENT))
                .accent
                .as_deref(),
            Some("#a6d8ff")
        );
        assert_eq!(
            resolve(enabled(ThemeMode::System), light, Some(ACCENT))
                .accent
                .as_deref(),
            Some("#005a9e")
        );
        assert_eq!(resolve(enabled(ThemeMode::Light), light, None).accent, None);
    }

    const WIN11_DARK: SystemAppearance = SystemAppearance {
        apps_light: false,
        taskbar_light: false,
        transparency: true,
        windows_11: true,
    };

    #[test]
    fn thm_001_system_mode_follows_the_apps_theme() {
        let light = SystemAppearance {
            apps_light: true,
            ..WIN11_DARK
        };

        assert_eq!(
            resolve(ui(ThemeMode::System), WIN11_DARK, None).theme,
            ThemeDto::Dark
        );
        assert_eq!(
            resolve(ui(ThemeMode::System), light, None).theme,
            ThemeDto::Light
        );
    }

    #[test]
    fn thm_001_explicit_modes_override_the_system() {
        assert_eq!(
            resolve(ui(ThemeMode::Light), WIN11_DARK, None).theme,
            ThemeDto::Light
        );
        let light = SystemAppearance {
            apps_light: true,
            ..WIN11_DARK
        };
        assert_eq!(
            resolve(ui(ThemeMode::Dark), light, None).theme,
            ThemeDto::Dark
        );
    }

    #[test]
    fn thm_002_acrylic_requires_windows_11_with_transparency() {
        assert_eq!(
            resolve(ui(ThemeMode::System), WIN11_DARK, None).backdrop,
            BackdropDto::Acrylic
        );
        let windows_10 = SystemAppearance {
            windows_11: false,
            ..WIN11_DARK
        };
        let opaque = SystemAppearance {
            transparency: false,
            ..WIN11_DARK
        };
        assert_eq!(
            resolve(ui(ThemeMode::System), windows_10, None).backdrop,
            BackdropDto::Solid
        );
        assert_eq!(
            resolve(ui(ThemeMode::System), opaque, None).backdrop,
            BackdropDto::Solid
        );
    }
}
