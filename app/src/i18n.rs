//! Native-side translations (tray menu, notifications).
//!
//! Shares the UI catalogs so there is a single source of strings (specs/06 §6, I18N-004).

use std::sync::{PoisonError, RwLock};

use clipvault_domain::settings::LanguagePreference;
use serde_json::Value;

const EN: &str = include_str!("../../ui/src/i18n/en.json");
const PT_BR: &str = include_str!("../../ui/src/i18n/pt-BR.json");

/// Supported interface languages (I18N-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    PtBr,
}

impl Locale {
    /// Maps a BCP 47 / Windows locale tag to a supported locale; English is the fallback.
    pub fn from_tag(tag: Option<&str>) -> Self {
        match tag.and_then(|t| t.split(['-', '_']).next()) {
            Some(lang) if lang.eq_ignore_ascii_case("pt") => Self::PtBr,
            _ => Self::En,
        }
    }

    /// The locale matching the Windows display language.
    pub fn system() -> Self {
        Self::from_tag(sys_locale::get_locale().as_deref())
    }

    /// The effective locale for the user's preference (I18N-001/002).
    pub fn resolve(preference: LanguagePreference, system: impl FnOnce() -> Self) -> Self {
        match preference {
            LanguagePreference::System => system(),
            LanguagePreference::PtBr => Self::PtBr,
            LanguagePreference::En => Self::En,
        }
    }

    /// BCP 47 tag sent to the UI.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-BR",
        }
    }

    fn catalog(self) -> &'static str {
        match self {
            Self::En => EN,
            Self::PtBr => PT_BR,
        }
    }
}

/// Resolves dotted keys (`tray.quit`) with fallback to English, then to the key itself.
/// The locale can change at runtime (I18N-002).
#[derive(Debug)]
pub struct Translator {
    state: RwLock<Catalogs>,
}

#[derive(Debug, Clone)]
struct Catalogs {
    locale: Locale,
    primary: Value,
    fallback: Value,
}

impl Catalogs {
    fn load(locale: Locale) -> Result<Self, serde_json::Error> {
        Ok(Self {
            locale,
            primary: serde_json::from_str(locale.catalog())?,
            fallback: serde_json::from_str(EN)?,
        })
    }
}

impl Translator {
    /// Loads the catalog for `locale`.
    ///
    /// # Errors
    /// Fails if an embedded catalog is not valid JSON.
    pub fn new(locale: Locale) -> Result<Self, serde_json::Error> {
        Ok(Self {
            state: RwLock::new(Catalogs::load(locale)?),
        })
    }

    /// The active locale.
    pub fn locale(&self) -> Locale {
        self.state
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .locale
    }

    /// Switches to `locale`; returns whether it changed.
    pub fn set_locale(&self, locale: Locale) -> bool {
        if self.locale() == locale {
            return false;
        }
        match Catalogs::load(locale) {
            Ok(catalogs) => {
                *self.state.write().unwrap_or_else(PoisonError::into_inner) = catalogs;
                true
            }
            Err(error) => {
                tracing::error!(%error, "translation catalog could not be loaded");
                false
            }
        }
    }

    /// Translates `key`.
    pub fn t(&self, key: &str) -> String {
        let state = self.state.read().unwrap_or_else(PoisonError::into_inner);
        lookup(&state.primary, key)
            .or_else(|| lookup(&state.fallback, key))
            .unwrap_or(key)
            .to_owned()
    }
}

fn lookup<'a>(tree: &'a Value, key: &str) -> Option<&'a str> {
    key.split('.')
        .try_fold(tree, |node, part| node.get(part))?
        .as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i18n_001_portuguese_tags_map_to_pt_br() {
        for tag in ["pt-BR", "pt", "pt_PT", "PT-br"] {
            assert_eq!(Locale::from_tag(Some(tag)), Locale::PtBr, "{tag}");
        }
    }

    #[test]
    fn i18n_001_other_or_missing_tags_fall_back_to_english() {
        for tag in [Some("en-US"), Some("fr-FR"), Some(""), None] {
            assert_eq!(Locale::from_tag(tag), Locale::En, "{tag:?}");
        }
    }

    #[test]
    fn translates_in_requested_locale() {
        let pt = Translator::new(Locale::PtBr).unwrap();
        let en = Translator::new(Locale::En).unwrap();

        assert_eq!(pt.t("tray.quit"), "Sair");
        assert_eq!(en.t("tray.quit"), "Quit");
    }

    #[test]
    fn missing_or_non_leaf_keys_return_the_key() {
        let en = Translator::new(Locale::En).unwrap();

        assert_eq!(en.t("does.not.exist"), "does.not.exist");
        assert_eq!(en.t("tray"), "tray");
    }

    #[test]
    fn falls_back_to_english_when_key_missing_in_locale() {
        let tr = Translator {
            state: RwLock::new(Catalogs {
                locale: Locale::PtBr,
                primary: serde_json::json!({}),
                fallback: serde_json::json!({ "a": { "b": "English" } }),
            }),
        };

        assert_eq!(tr.t("a.b"), "English");
    }

    #[test]
    fn i18n_002_explicit_preferences_override_the_system_language() {
        let system = || Locale::PtBr;

        assert_eq!(
            Locale::resolve(LanguagePreference::System, system),
            Locale::PtBr
        );
        assert_eq!(Locale::resolve(LanguagePreference::En, system), Locale::En);
        assert_eq!(
            Locale::resolve(LanguagePreference::PtBr, || Locale::En),
            Locale::PtBr
        );
    }

    #[test]
    fn i18n_002_switching_the_locale_changes_translations_without_restarting() {
        let tr = Translator::new(Locale::En).unwrap();

        assert!(tr.set_locale(Locale::PtBr));
        assert!(!tr.set_locale(Locale::PtBr), "no change reported twice");

        assert_eq!(tr.locale(), Locale::PtBr);
        assert_eq!(tr.t("tray.quit"), "Sair");
        assert_eq!((Locale::PtBr.tag(), Locale::En.tag()), ("pt-BR", "en"));
    }
}
