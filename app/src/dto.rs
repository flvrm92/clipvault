//! Data transfer objects exchanged with the UI (pattern: DTO + Mapper — specs/04 §P-09).
//!
//! TypeScript definitions are generated into `ui/src/api/bindings/` by `cargo test`; CI fails
//! when the committed bindings are out of date.

use clipvault_application::ports::{
    ClearScope, HistoryQuery, KindFilter, Page, PortError, Suspension,
};
use clipvault_application::use_cases::{PasteMode, PasteOutcome, SnippetDraft};
use clipvault_application::{AppError, HotkeyConflict};
use clipvault_domain::clip::{ClipContent, ClipItem};
use clipvault_domain::hotkey::HotkeyError;
use clipvault_domain::ids::SnippetId;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AppearanceDto {
    pub theme: ThemeDto,
    pub backdrop: BackdropDto,
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
