//! Data transfer objects exchanged with the UI (pattern: DTO + Mapper — specs/04 §P-09).
//!
//! TypeScript definitions are generated into `ui/src/api/bindings/` by `cargo test`; CI fails
//! when the committed bindings are out of date.

use clipvault_application::ports::{
    ClearScope, HistoryQuery, KindFilter, Page, PortError, StartupState, Suspension,
};
use clipvault_application::use_cases::{PasteMode, PasteOutcome, SnippetDraft};
use clipvault_application::{AppError, HotkeyConflict};
use clipvault_domain::clip::{ClipContent, ClipItem};
use clipvault_domain::hotkey::HotkeyError;
use clipvault_domain::ids::SnippetId;
use clipvault_domain::retention::{MaxItems, RetentionPeriod, RetentionPolicy};
use clipvault_domain::settings::{
    CaptureSettings, LanguagePreference, Settings, SettingsSection, ThemeMode,
};
use clipvault_domain::snippet::{Snippet, SnippetError};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Kind of a history item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClipKindDto {
    Text,
    Image,
}

/// A history row (HIST-001). Images are fetched through `clip://thumb/<id>` (SEC-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ClipItemDto {
    #[ts(type = "number")]
    pub id: i64,
    pub kind: ClipKindDto,
    /// Text preview (≤ 3 lines); empty for images.
    pub preview: String,
    pub pinned: bool,
    /// Unix epoch milliseconds.
    #[ts(type = "number")]
    pub last_used_at: i64,
    pub source_app: Option<String>,
    /// Image dimensions; `None` for text.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl From<&ClipItem> for ClipItemDto {
    fn from(item: &ClipItem) -> Self {
        let (kind, preview, size) = match &item.content {
            ClipContent::Text(text) => (ClipKindDto::Text, text.preview(), None),
            ClipContent::Image(meta) => (
                ClipKindDto::Image,
                String::new(),
                Some((meta.width, meta.height)),
            ),
        };
        Self {
            id: item.id.get(),
            kind,
            preview,
            pinned: item.pinned,
            last_used_at: item.last_used_at.as_millis(),
            source_app: item.source_app.as_ref().map(|a| a.as_str().to_owned()),
            width: size.map(|(w, _)| w),
            height: size.map(|(_, h)| h),
        }
    }
}

/// A page of history rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryPageDto {
    pub items: Vec<ClipItemDto>,
    pub has_more: bool,
}

impl From<Page<ClipItem>> for HistoryPageDto {
    fn from(page: Page<ClipItem>) -> Self {
        Self {
            items: page.items.iter().map(ClipItemDto::from).collect(),
            has_more: page.has_more,
        }
    }
}

/// History filter (HIST-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum KindFilterDto {
    All,
    Text,
    Images,
    Pinned,
}

/// A page request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HistoryQueryDto {
    pub filter: KindFilterDto,
    pub search: Option<String>,
    pub offset: u32,
    pub limit: u32,
}

impl From<HistoryQueryDto> for HistoryQuery {
    fn from(dto: HistoryQueryDto) -> Self {
        Self {
            filter: match dto.filter {
                KindFilterDto::All => KindFilter::All,
                KindFilterDto::Text => KindFilter::Text,
                KindFilterDto::Images => KindFilter::Images,
                KindFilterDto::Pinned => KindFilter::Pinned,
            },
            search: dto.search,
            offset: dto.offset as usize,
            limit: dto.limit as usize,
        }
    }
}

/// How text is pasted (PST-001, PST-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PasteModeDto {
    AsIs,
    Trimmed,
}

impl From<PasteModeDto> for PasteMode {
    fn from(dto: PasteModeDto) -> Self {
        match dto {
            PasteModeDto::AsIs => Self::AsIs,
            PasteModeDto::Trimmed => Self::TrimmedText,
        }
    }
}

/// Result of a paste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PasteResultDto {
    Pasted,
    /// Left on the clipboard (PST-005/006); the user is notified.
    CopiedOnly,
}

impl From<PasteOutcome> for PasteResultDto {
    fn from(outcome: PasteOutcome) -> Self {
        match outcome {
            PasteOutcome::Pasted => Self::Pasted,
            PasteOutcome::CopiedOnly(_) => Self::CopiedOnly,
        }
    }
}

/// What "clear history" removes (HIST-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ClearScopeDto {
    UnpinnedOnly,
    Everything,
}

impl From<ClearScopeDto> for ClearScope {
    fn from(dto: ClearScopeDto) -> Self {
        match dto {
            ClearScopeDto::UnpinnedOnly => Self::UnpinnedOnly,
            ClearScopeDto::Everything => Self::Everything,
        }
    }
}

/// Effective color theme (THM-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ThemeDto {
    Light,
    Dark,
}

/// Window background (THM-002): translucent Acrylic on Windows 11 with transparency enabled,
/// otherwise a solid color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BackdropDto {
    Acrylic,
    Solid,
}

/// Appearance applied by the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppearanceDto {
    pub theme: ThemeDto,
    pub backdrop: BackdropDto,
    /// Windows accent color for this theme (`#rrggbb`) when the user enabled it (THM-004).
    pub accent: Option<String>,
}

/// State shown by the popup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PopupStateDto {
    /// Shortcut that opens the popup, in canonical form (HIST-010).
    pub open_shortcut: String,
    pub capture_paused: bool,
    pub appearance: AppearanceDto,
    /// Effective interface language, resolved by the backend (I18N-002).
    pub locale: String,
}

/// Error category; the UI maps it to a translated message (specs/03 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ErrorCodeDto {
    NotFound,
    Unavailable,
    Storage,
    /// Taken by another application (HK-004) or a duplicate.
    InUse,
    /// Snippet name already used (SNP-001).
    NameTaken,
    /// Shortcut used by the open-popup shortcut (HK-003).
    HotkeyTakenByOpen,
    /// Shortcut used by a snippet; `detail` is its name (HK-003).
    HotkeyTakenBySnippet,
    /// Invalid snippet field; `detail` says which rule (SNP-001).
    InvalidSnippet,
    /// Invalid shortcut; `detail` says which rule (HK-003).
    InvalidHotkey,
    /// A setting value that is not offered (CAP-006, RET-001/002).
    InvalidSetting,
    /// A blank application name (CAP-009).
    InvalidAppName,
}

/// Error returned by every command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandError {
    pub code: ErrorCodeDto,
    /// Extra data for the message: a snippet name or the violated rule.
    pub detail: Option<String>,
    /// Diagnostic detail (English, never clipboard content).
    pub message: String,
}

fn snippet_rule(error: &SnippetError) -> &'static str {
    match error {
        SnippetError::NameBlank => "nameBlank",
        SnippetError::NameTooLong { .. } => "nameTooLong",
        SnippetError::BodyBlank => "bodyBlank",
        SnippetError::BodyTooLarge { .. } => "bodyTooLarge",
    }
}

fn hotkey_rule(error: &HotkeyError) -> &'static str {
    match error {
        HotkeyError::Empty => "empty",
        HotkeyError::UnknownToken(_) => "unknownToken",
        HotkeyError::DuplicateModifier => "duplicateModifier",
        HotkeyError::MissingKey => "missingKey",
        HotkeyError::MultipleKeys => "multipleKeys",
        HotkeyError::MissingModifier => "missingModifier",
        HotkeyError::Reserved => "reserved",
    }
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        let (code, detail) = match &error {
            AppError::Port(PortError::NotFound) => (ErrorCodeDto::NotFound, None),
            AppError::Port(PortError::Unavailable(_)) => (ErrorCodeDto::Unavailable, None),
            AppError::Port(PortError::Storage(_)) => (ErrorCodeDto::Storage, None),
            AppError::Port(PortError::InUse(_)) => (ErrorCodeDto::InUse, None),
            AppError::NameTaken => (ErrorCodeDto::NameTaken, None),
            AppError::HotkeyTaken(HotkeyConflict::OpenHistory) => {
                (ErrorCodeDto::HotkeyTakenByOpen, None)
            }
            AppError::HotkeyTaken(HotkeyConflict::Snippet(name)) => {
                (ErrorCodeDto::HotkeyTakenBySnippet, Some(name.clone()))
            }
            AppError::InvalidSnippet(rule) => (
                ErrorCodeDto::InvalidSnippet,
                Some(snippet_rule(rule).to_owned()),
            ),
            AppError::InvalidHotkey(rule) => (
                ErrorCodeDto::InvalidHotkey,
                Some(hotkey_rule(rule).to_owned()),
            ),
            AppError::InvalidSetting(_) => (ErrorCodeDto::InvalidSetting, None),
            AppError::InvalidAppName(_) => (ErrorCodeDto::InvalidAppName, None),
        };
        Self {
            code,
            detail,
            message: error.to_string(),
        }
    }
}

/// A snippet (SNP-001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnippetDto {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    pub body: String,
    /// Canonical shortcut text, e.g. `"Ctrl+Shift+1"`.
    pub hotkey: Option<String>,
    pub enabled: bool,
}

impl From<&Snippet> for SnippetDto {
    fn from(snippet: &Snippet) -> Self {
        Self {
            id: snippet.id.get(),
            name: snippet.name.as_str().to_owned(),
            body: snippet.body.as_str().to_owned(),
            hotkey: snippet.hotkey.map(|h| h.to_string()),
            enabled: snippet.enabled,
        }
    }
}

/// A snippet as edited by the user; validated by the backend.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SnippetDraftDto {
    /// `null` creates a snippet.
    #[ts(type = "number | null")]
    pub id: Option<i64>,
    pub name: String,
    pub body: String,
    pub hotkey: Option<String>,
    pub enabled: bool,
}

impl From<SnippetDraftDto> for SnippetDraft {
    fn from(dto: SnippetDraftDto) -> Self {
        Self {
            id: dto.id.map(SnippetId::new),
            name: dto.name,
            body: dto.body,
            hotkey: dto.hotkey,
            enabled: dto.enabled,
        }
    }
}

/// The "Atalhos" settings section (HK-001, SNP-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ShortcutSettingsDto {
    pub open_shortcut: String,
    pub restore_clipboard: bool,
    pub snippets: Vec<SnippetDto>,
}

/// Whether global shortcuts are active while one is being recorded (HK-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SuspensionDto {
    Active,
    Suspended,
}

impl From<SuspensionDto> for Suspension {
    fn from(dto: SuspensionDto) -> Self {
        match dto {
            SuspensionDto::Active => Self::Active,
            SuspensionDto::Suspended => Self::Suspended,
        }
    }
}

/// Interface language preference (I18N-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LanguageDto {
    System,
    PtBr,
    En,
}

impl From<LanguagePreference> for LanguageDto {
    fn from(value: LanguagePreference) -> Self {
        match value {
            LanguagePreference::System => Self::System,
            LanguagePreference::PtBr => Self::PtBr,
            LanguagePreference::En => Self::En,
        }
    }
}

impl From<LanguageDto> for LanguagePreference {
    fn from(value: LanguageDto) -> Self {
        match value {
            LanguageDto::System => Self::System,
            LanguageDto::PtBr => Self::PtBr,
            LanguageDto::En => Self::En,
        }
    }
}

/// Theme preference (THM-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ThemeModeDto {
    System,
    Light,
    Dark,
}

impl From<ThemeMode> for ThemeModeDto {
    fn from(value: ThemeMode) -> Self {
        match value {
            ThemeMode::System => Self::System,
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
        }
    }
}

impl From<ThemeModeDto> for ThemeMode {
    fn from(value: ThemeModeDto) -> Self {
        match value {
            ThemeModeDto::System => Self::System,
            ThemeModeDto::Light => Self::Light,
            ThemeModeDto::Dark => Self::Dark,
        }
    }
}

/// Retention policy (RET-001/002); `null` means no limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RetentionDto {
    pub max_age_days: Option<u32>,
    pub max_items: Option<u32>,
}

impl From<RetentionPolicy> for RetentionDto {
    fn from(policy: RetentionPolicy) -> Self {
        Self {
            max_age_days: policy.period.days(),
            max_items: policy.max_items.limit(),
        }
    }
}

impl TryFrom<RetentionDto> for RetentionPolicy {
    type Error = CommandError;

    fn try_from(dto: RetentionDto) -> Result<Self, Self::Error> {
        let invalid = |error: clipvault_domain::retention::RetentionError| CommandError {
            code: ErrorCodeDto::InvalidSetting,
            detail: None,
            message: error.to_string(),
        };
        Ok(Self {
            period: RetentionPeriod::from_days(dto.max_age_days).map_err(invalid)?,
            max_items: MaxItems::from_count(dto.max_items).map_err(invalid)?,
        })
    }
}

/// Values the settings window offers in its selects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsOptionsDto {
    pub max_age_days: Vec<Option<u32>>,
    pub max_items: Vec<Option<u32>>,
    #[ts(type = "number[]")]
    pub text_limits: Vec<u64>,
    #[ts(type = "number[]")]
    pub image_limits: Vec<u64>,
    /// Restored by "Restore defaults" in History; previewed before it removes items (RET-004).
    pub default_retention: RetentionDto,
}

impl SettingsOptionsDto {
    /// The options defined by the domain (RET-001/002, CAP-006).
    pub fn offered() -> Self {
        Self {
            max_age_days: RetentionPeriod::ALL.iter().map(|p| p.days()).collect(),
            max_items: MaxItems::ALL.iter().map(|m| m.limit()).collect(),
            text_limits: CaptureSettings::TEXT_LIMIT_OPTIONS.to_vec(),
            image_limits: CaptureSettings::IMAGE_LIMIT_OPTIONS.to_vec(),
            default_retention: RetentionPolicy::default().into(),
        }
    }
}

/// Every setting shown in the General, History, Privacy and Appearance sections (CFG-001).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SettingsDto {
    /// Effective interface language (I18N-002).
    pub locale: String,
    pub language: LanguageDto,
    pub capture_paused: bool,
    pub retention: RetentionDto,
    pub clear_on_exit: bool,
    #[ts(type = "number")]
    pub max_text_bytes: u64,
    #[ts(type = "number")]
    pub max_image_bytes: u64,
    pub excluded_apps: Vec<String>,
    pub theme: ThemeModeDto,
    pub use_accent_color: bool,
    pub options: SettingsOptionsDto,
}

impl SettingsDto {
    /// Maps `settings` with the effective `locale` tag.
    pub fn new(settings: &Settings, locale: &str) -> Self {
        Self {
            locale: locale.to_owned(),
            language: settings.ui.language.into(),
            capture_paused: settings.capture.paused,
            retention: settings.retention.policy.into(),
            clear_on_exit: settings.retention.clear_on_exit,
            max_text_bytes: settings.capture.max_text_bytes,
            max_image_bytes: settings.capture.max_image_bytes,
            excluded_apps: settings
                .capture
                .excluded_apps
                .iter()
                .map(|a| a.as_str().to_owned())
                .collect(),
            theme: settings.ui.theme.into(),
            use_accent_color: settings.ui.use_accent_color,
            options: SettingsOptionsDto::offered(),
        }
    }
}

/// Which size limit to change (CAP-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SizeLimitDto {
    Text,
    Image,
}

/// A section with a "Restore defaults" button (CFG-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SettingsSectionDto {
    General,
    History,
    Shortcuts,
    Privacy,
    Appearance,
}

impl From<SettingsSectionDto> for SettingsSection {
    fn from(dto: SettingsSectionDto) -> Self {
        match dto {
            SettingsSectionDto::General => Self::General,
            SettingsSectionDto::History => Self::History,
            SettingsSectionDto::Shortcuts => Self::Shortcuts,
            SettingsSectionDto::Privacy => Self::Privacy,
            SettingsSectionDto::Appearance => Self::Appearance,
        }
    }
}

/// The "Sobre" section (CFG-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AboutDto {
    pub version: String,
}

/// Links of the "Sobre" section; the URLs stay in the backend (CFG-005, SEC-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AboutLinkDto {
    Privacy,
    Licenses,
    Support,
}

impl AboutLinkDto {
    /// The fixed address of the link.
    pub const fn url(self) -> &'static str {
        match self {
            Self::Privacy => "https://github.com/flvrm92/clipvault/blob/main/PRIVACY.md",
            Self::Licenses => {
                "https://github.com/flvrm92/clipvault/blob/main/THIRD-PARTY-NOTICES.md"
            }
            Self::Support => "https://github.com/flvrm92/clipvault/issues",
        }
    }
}

/// Start-with-Windows state (CFG-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum StartupStateDto {
    Enabled,
    Disabled,
    DisabledByUser,
    DisabledByPolicy,
    EnabledByPolicy,
}

impl From<StartupState> for StartupStateDto {
    fn from(state: StartupState) -> Self {
        match state {
            StartupState::Enabled => Self::Enabled,
            StartupState::Disabled => Self::Disabled,
            StartupState::DisabledByUser => Self::DisabledByUser,
            StartupState::DisabledByPolicy => Self::DisabledByPolicy,
            StartupState::EnabledByPolicy => Self::EnabledByPolicy,
        }
    }
}

/// What the welcome window shows (TRAY-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WelcomeDto {
    pub open_shortcut: String,
    pub startup: StartupStateDto,
    pub appearance: AppearanceDto,
    pub locale: String,
}

#[cfg(test)]
mod tests {
    use clipvault_domain::app_name::AppName;
    use clipvault_domain::hash::ContentHash;
    use clipvault_domain::ids::ClipId;
    use clipvault_domain::image::{FileKey, ImageMeta};
    use clipvault_domain::text::ClipText;
    use clipvault_domain::time::Timestamp;
    use serde_json::json;

    use super::*;

    fn item(content: ClipContent) -> ClipItem {
        ClipItem {
            id: ClipId::new(7),
            content,
            hash: ContentHash::of_text("x"),
            source_app: Some(AppName::parse("code.exe").unwrap()),
            pinned: true,
            use_count: 3,
            created_at: Timestamp::from_millis(1),
            last_used_at: Timestamp::from_millis(1_700_000_000_000),
        }
    }

    #[test]
    fn hist_001_text_items_serialize_with_their_preview() {
        let dto = ClipItemDto::from(&item(ClipContent::Text(ClipText::new("a\n\n b").unwrap())));

        assert_eq!(
            serde_json::to_value(dto).unwrap(),
            json!({
                "id": 7, "kind": "text", "preview": "a\nb", "pinned": true,
                "lastUsedAt": 1_700_000_000_000_i64, "sourceApp": "code.exe",
                "width": null, "height": null
            })
        );
    }

    #[test]
    fn hist_001_image_items_serialize_dimensions_but_no_path() {
        let meta = ImageMeta {
            file_key: FileKey::from_hash(&ContentHash::of_text("img")),
            width: 1920,
            height: 1080,
            file_size: 10,
            thumb_width: 320,
            thumb_height: 180,
        };

        let value =
            serde_json::to_value(ClipItemDto::from(&item(ClipContent::Image(meta)))).unwrap();

        assert_eq!(value["kind"], "image");
        assert_eq!(
            (value["width"].clone(), value["height"].clone()),
            (json!(1920), json!(1080))
        );
        assert!(
            !value.to_string().contains("fileKey"),
            "SEC-003: no file identifiers in the UI"
        );
    }

    #[test]
    fn queries_and_modes_deserialize_from_camel_case() {
        let query: HistoryQueryDto = serde_json::from_value(
            json!({"filter": "images", "search": "café", "offset": 50, "limit": 50}),
        )
        .unwrap();
        let mode: PasteModeDto = serde_json::from_value(json!("trimmed")).unwrap();

        let query = HistoryQuery::from(query);
        assert_eq!(query.filter, KindFilter::Images);
        assert_eq!((query.offset, query.limit), (50, 50));
        assert_eq!(PasteMode::from(mode), PasteMode::TrimmedText);
    }

    #[test]
    fn errors_carry_a_code_for_translation() {
        let error = CommandError::from(AppError::Port(PortError::NotFound));

        assert_eq!(
            serde_json::to_value(error).unwrap(),
            json!({"code": "notFound", "detail": null, "message": "not found"})
        );
    }

    #[test]
    fn hk_003_conflicts_and_rules_are_detailed_for_the_ui() {
        let by_snippet = CommandError::from(AppError::HotkeyTaken(HotkeyConflict::Snippet(
            "Assinatura".into(),
        )));
        let reserved = CommandError::from(AppError::InvalidHotkey(HotkeyError::Reserved));
        let blank = CommandError::from(AppError::InvalidSnippet(SnippetError::NameBlank));
        let in_use = CommandError::from(AppError::Port(PortError::InUse("x".into())));

        assert_eq!(
            (by_snippet.code, by_snippet.detail.as_deref()),
            (ErrorCodeDto::HotkeyTakenBySnippet, Some("Assinatura"))
        );
        assert_eq!(reserved.detail.as_deref(), Some("reserved"));
        assert_eq!(blank.detail.as_deref(), Some("nameBlank"));
        assert_eq!(in_use.code, ErrorCodeDto::InUse);
    }

    #[test]
    fn cfg_001_settings_serialize_with_the_offered_options() {
        let value = serde_json::to_value(SettingsDto::new(&Settings::default(), "pt-BR")).unwrap();

        assert_eq!(value["locale"], "pt-BR");
        assert_eq!(value["language"], "system");
        assert_eq!(
            value["retention"],
            json!({"maxAgeDays": 30, "maxItems": 1000})
        );
        assert_eq!(
            value["options"]["maxAgeDays"],
            json!([1, 7, 30, 90, 365, null])
        );
        assert_eq!(
            value["options"]["maxItems"],
            json!([100, 500, 1000, 5000, null])
        );
        assert_eq!(value["maxTextBytes"], json!(1_048_576));
        assert_eq!(value["theme"], "system");
    }

    #[test]
    fn ret_001_only_offered_retention_values_are_accepted() {
        let valid = RetentionDto {
            max_age_days: None,
            max_items: Some(5_000),
        };
        let invalid = RetentionDto {
            max_age_days: Some(2),
            max_items: Some(5_000),
        };

        assert_eq!(
            RetentionPolicy::try_from(valid).unwrap(),
            RetentionPolicy {
                period: RetentionPeriod::Forever,
                max_items: MaxItems::FiveThousand
            }
        );
        assert_eq!(
            RetentionPolicy::try_from(invalid).unwrap_err().code,
            ErrorCodeDto::InvalidSetting
        );
    }

    #[test]
    fn cfg_005_about_links_point_to_fixed_https_addresses() {
        for link in [
            AboutLinkDto::Privacy,
            AboutLinkDto::Licenses,
            AboutLinkDto::Support,
        ] {
            assert!(
                link.url()
                    .starts_with("https://github.com/flvrm92/clipvault/")
            );
        }
        let link: AboutLinkDto = serde_json::from_value(json!("support")).unwrap();
        assert_eq!(link, AboutLinkDto::Support);
    }

    #[test]
    fn cfg_003_startup_states_serialize_in_camel_case() {
        let states = [
            (StartupState::Enabled, "enabled"),
            (StartupState::Disabled, "disabled"),
            (StartupState::DisabledByUser, "disabledByUser"),
            (StartupState::DisabledByPolicy, "disabledByPolicy"),
            (StartupState::EnabledByPolicy, "enabledByPolicy"),
        ];
        for (state, name) in states {
            assert_eq!(
                serde_json::to_value(StartupStateDto::from(state)).unwrap(),
                name
            );
        }
    }

    #[test]
    fn i18n_002_language_and_theme_round_trip() {
        for language in [LanguageDto::System, LanguageDto::PtBr, LanguageDto::En] {
            assert_eq!(
                LanguageDto::from(LanguagePreference::from(language)),
                language
            );
        }
        for theme in [
            ThemeModeDto::System,
            ThemeModeDto::Light,
            ThemeModeDto::Dark,
        ] {
            assert_eq!(ThemeModeDto::from(ThemeMode::from(theme)), theme);
        }
        let language: LanguageDto = serde_json::from_value(json!("ptBr")).unwrap();
        assert_eq!(language, LanguageDto::PtBr);
    }

    #[test]
    fn snippet_drafts_deserialize_and_snippets_serialize() {
        let draft: SnippetDraftDto = serde_json::from_value(json!({
            "id": null, "name": "Teste", "body": "Teste de texto",
            "hotkey": "Ctrl+Shift+V", "enabled": true
        }))
        .unwrap();

        let draft = SnippetDraft::from(draft);
        assert_eq!(draft.id, None);
        assert_eq!(draft.hotkey.as_deref(), Some("Ctrl+Shift+V"));
    }
}
