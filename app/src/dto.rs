//! Data transfer objects exchanged with the UI (pattern: DTO + Mapper — specs/04 §P-09).
//!
//! TypeScript definitions are generated into `ui/src/api/bindings/` by `cargo test`; CI fails
//! when the committed bindings are out of date.

use clipvault_application::AppError;
use clipvault_application::ports::{ClearScope, HistoryQuery, KindFilter, Page, PortError};
use clipvault_application::use_cases::{PasteMode, PasteOutcome};
use clipvault_domain::clip::{ClipContent, ClipItem};
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
}

/// Error returned by every command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CommandError {
    pub code: ErrorCodeDto,
    /// Diagnostic detail (English, never clipboard content).
    pub message: String,
}

impl From<AppError> for CommandError {
    fn from(error: AppError) -> Self {
        let AppError::Port(port) = &error;
        let code = match port {
            PortError::NotFound => ErrorCodeDto::NotFound,
            PortError::Unavailable(_) => ErrorCodeDto::Unavailable,
            PortError::Storage(_) => ErrorCodeDto::Storage,
        };
        Self {
            code,
            message: error.to_string(),
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
            json!({"code": "notFound", "message": "not found"})
        );
    }
}
