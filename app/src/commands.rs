//! Tauri commands: DTO in → use case → DTO out. No business rules (specs/03 §3.4).
// Tauri injects `State`/`AppHandle` by value; that is the framework's calling convention.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use clipvault_application::AppError;
use clipvault_application::use_cases::{
    CaptureState, ClearHistory, CopyItem, DeleteItem, GetItem, ListHistory, PasteItem,
    PasteOutcome, SetCaptureState, TogglePin,
};
use clipvault_domain::clip::{ClipContent, PinState};
use clipvault_domain::ids::ClipId;
use serde::Deserialize;
use tauri::{AppHandle, Manager, State, Wry};
use tauri_plugin_notification::NotificationExt;
use ts_rs::TS;

use crate::appearance;
use crate::container::Container;
use crate::dto::{
    ClearScopeDto, CommandError, HistoryPageDto, HistoryQueryDto, PasteModeDto, PasteResultDto,
    PopupStateDto,
};
use crate::i18n::Translator;
use crate::popup::{self, PopupState};

type CommandResult<T> = Result<T, CommandError>;

/// Time for the hidden popup to give up the foreground before the target is restored.
const HIDE_SETTLE: Duration = Duration::from_millis(40);

/// Capture state as sent by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaptureStateDto {
    Active,
    Paused,
}

/// HIST-001..004: a page of the history.
#[tauri::command]
pub fn list_history(
    container: State<'_, Container>,
    query: HistoryQueryDto,
) -> CommandResult<HistoryPageDto> {
    let page = ListHistory::new(container.history.clone()).execute(query.into())?;
    Ok(page.into())
}

/// HIST-008: the full text of an item, for the preview (empty for images).
#[tauri::command]
pub fn get_item_text(container: State<'_, Container>, id: i64) -> CommandResult<String> {
    let item = GetItem::new(container.history.clone()).execute(ClipId::new(id))?;
    Ok(match item.content {
        ClipContent::Text(text) => text.as_str().to_owned(),
        ClipContent::Image(_) => String::new(),
    })
}

/// PST-001..006: hides the popup and pastes into the window that was active before it.
#[tauri::command]
pub async fn paste_item(
    app: AppHandle<Wry>,
    id: i64,
    mode: PasteModeDto,
) -> CommandResult<PasteResultDto> {
    popup::hide(&app);
    let target = app.state::<PopupState>().take_target();
    let deps = app.state::<Container>().paste_dependencies();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(HIDE_SETTLE);
        PasteItem::new(deps).execute(ClipId::new(id), mode.into(), target.as_ref())
    })
    .await
    .map_err(|error| {
        CommandError::from(AppError::Port(
            clipvault_application::ports::PortError::Unavailable(error.to_string()),
        ))
    })??;
    if let PasteOutcome::CopiedOnly(reason) = outcome {
        tracing::info!(?reason, "item copied; user pastes manually");
        notify_copied_only(&app);
    }
    Ok(outcome.into())
}

fn notify_copied_only(app: &AppHandle<Wry>) {
    let body = app.state::<Translator>().t("notification.copiedOnly");
    if let Err(error) = app
        .notification()
        .builder()
        .title("ClipVault")
        .body(body)
        .show()
    {
        tracing::warn!(%error, "notification could not be shown");
    }
}

/// HIST-011: copies an item without pasting and closes the popup.
#[tauri::command]
pub fn copy_item(app: AppHandle<Wry>, id: i64) -> CommandResult<()> {
    popup::hide(&app);
    let deps = app.state::<Container>().paste_dependencies();
    CopyItem::new(deps).execute(ClipId::new(id))?;
    Ok(())
}

/// HIST-005: pins or unpins; returns whether the item is now pinned.
#[tauri::command]
pub fn toggle_pin(container: State<'_, Container>, id: i64) -> CommandResult<bool> {
    let state = TogglePin::new(container.history.clone(), container.events.clone())
        .execute(ClipId::new(id))?;
    Ok(state == PinState::Pinned)
}

/// HIST-006: deletes an item (and its image file).
#[tauri::command]
pub fn delete_item(container: State<'_, Container>, id: i64) -> CommandResult<()> {
    DeleteItem::new(
        container.history.clone(),
        container.images.clone(),
        container.events.clone(),
    )
    .execute(ClipId::new(id))?;
    Ok(())
}

/// HIST-007: clears the history; returns how many items were removed.
#[tauri::command]
pub fn clear_history(container: State<'_, Container>, scope: ClearScopeDto) -> CommandResult<u32> {
    let removed = ClearHistory::new(
        container.history.clone(),
        container.images.clone(),
        container.events.clone(),
    )
    .execute(scope.into())?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

/// CAP-008: pauses or resumes capture.
#[tauri::command]
pub fn set_capture_state(
    container: State<'_, Container>,
    state: CaptureStateDto,
) -> CommandResult<()> {
    let state = match state {
        CaptureStateDto::Active => CaptureState::Active,
        CaptureStateDto::Paused => CaptureState::Paused,
    };
    SetCaptureState::new(container.settings.clone(), container.events.clone()).execute(state)?;
    Ok(())
}

/// State displayed by the popup: shortcut, capture state and appearance.
#[tauri::command]
pub fn get_popup_state(container: State<'_, Container>) -> CommandResult<PopupStateDto> {
    use clipvault_application::ports::SettingsRepository;
    let settings = container.settings.load().map_err(AppError::from)?;
    Ok(PopupStateDto {
        open_shortcut: settings.hotkeys.open.to_string(),
        capture_paused: settings.capture.paused,
        appearance: appearance::current(settings.ui.theme),
    })
}

/// PST-007: closes the popup without pasting.
#[tauri::command]
pub fn hide_popup(app: AppHandle<Wry>) {
    popup::hide(&app);
}
