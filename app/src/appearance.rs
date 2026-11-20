//! Effective theme and window backdrop (THM-001..003).

use clipvault_domain::settings::ThemeMode;
use clipvault_infrastructure::win32::{SystemAppearance, system_appearance};
use tauri::utils::config::WindowEffectsConfig;
use tauri::window::Effect;
use tauri::{WebviewWindow, Wry};

use crate::dto::{AppearanceDto, BackdropDto, ThemeDto};

/// Resolves the user preference against the system settings.
pub fn resolve(mode: ThemeMode, system: SystemAppearance) -> AppearanceDto {
    let theme = match (mode, system.apps_light) {
        (ThemeMode::Light, _) | (ThemeMode::System, true) => ThemeDto::Light,
        (ThemeMode::Dark, _) | (ThemeMode::System, false) => ThemeDto::Dark,
    };
    let backdrop = if system.windows_11 && system.transparency {
        BackdropDto::Acrylic
    } else {
        BackdropDto::Solid
    };
    AppearanceDto { theme, backdrop }
}

/// The appearance for the current settings.
pub fn current(mode: ThemeMode) -> AppearanceDto {
    resolve(mode, system_appearance())
}

/// Applies the backdrop to a window created with `transparent: true` (THM-002).
pub fn apply_backdrop(window: &WebviewWindow<Wry>, appearance: AppearanceDto) {
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

        assert_eq!(resolve(ThemeMode::System, WIN11_DARK).theme, ThemeDto::Dark);
        assert_eq!(resolve(ThemeMode::System, light).theme, ThemeDto::Light);
    }

    #[test]
    fn thm_001_explicit_modes_override_the_system() {
        assert_eq!(resolve(ThemeMode::Light, WIN11_DARK).theme, ThemeDto::Light);
        let light = SystemAppearance {
            apps_light: true,
            ..WIN11_DARK
        };
        assert_eq!(resolve(ThemeMode::Dark, light).theme, ThemeDto::Dark);
    }

    #[test]
    fn thm_002_acrylic_requires_windows_11_with_transparency() {
        assert_eq!(
            resolve(ThemeMode::System, WIN11_DARK).backdrop,
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
            resolve(ThemeMode::System, windows_10).backdrop,
            BackdropDto::Solid
        );
        assert_eq!(
            resolve(ThemeMode::System, opaque).backdrop,
            BackdropDto::Solid
        );
    }
}
