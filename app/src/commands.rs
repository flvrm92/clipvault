//! Tauri commands: DTO in → use case → DTO out. No business rules (specs/03 §3.4).
// Tauri injects `State`/`AppHandle` by value; that is the framework's calling convention.
#![allow(clippy::needless_pass_by_value)]

use std::time::Duration;

use clipvault_application::AppError;
use clipvault_application::use_cases::{
    CaptureState, ChangeOpenHotkey, ClearHistory, CopyItem, DeleteItem, DeleteSnippet, GetItem,
    ListHistory, ListSnippets, PasteItem, PasteOutcome, PasteSnippet, SaveSnippet, SetCaptureState,
    SnippetTarget, SuspendHotkeys, TogglePin, UpdateSettings,
};
use clipvault_domain::clip::{ClipContent, PinState};
use clipvault_domain::ids::{ClipId, SnippetId};
use serde::Deserialize;
use tauri::{AppHandle, Manager, State, Wry};
use ts_rs::TS;

use crate::container::Container;
use crate::dto::{
    ClearScopeDto, CommandError, HistoryPageDto, HistoryQueryDto, PasteModeDto, PasteResultDto,
    PopupStateDto, ShortcutSettingsDto, SnippetDraftDto, SnippetDto, SuspensionDto,
};
use crate::popup::{self, PopupState};
use crate::settings_window::Page;
use crate::{appearance, notify, preferences, settings_window};

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
    .map_err(|e| blocking_failed(&e))??;
    if let PasteOutcome::CopiedOnly(reason) = outcome {
        tracing::info!(?reason, "item copied; user pastes manually");
        notify::copied_only(&app);
    }
    Ok(outcome.into())
}

fn blocking_failed(error: &tauri::Error) -> CommandError {
    CommandError::from(AppError::Port(
        clipvault_application::ports::PortError::Unavailable(error.to_string()),
    ))
}

/// SNP-007: pastes a snippet from the popup into the window that was active before it.
#[tauri::command]
pub async fn paste_snippet(app: AppHandle<Wry>, id: i64) -> CommandResult<PasteResultDto> {
    popup::hide(&app);
    let target = app.state::<PopupState>().take_target();
    let deps = app.state::<Container>().paste_snippet_dependencies();
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(HIDE_SETTLE);
        PasteSnippet::new(deps).execute(SnippetId::new(id), SnippetTarget::Window(target))
    })
    .await
    .map_err(|e| blocking_failed(&e))??;
    if let PasteOutcome::CopiedOnly(_) = outcome {
        notify::copied_only(&app);
    }
    Ok(outcome.into())
}

/// SNP-001/007: every snippet in display order.
#[tauri::command]
pub fn list_snippets(container: State<'_, Container>) -> CommandResult<Vec<SnippetDto>> {
    let snippets = ListSnippets::new(container.snippets.clone()).execute()?;
    Ok(snippets.iter().map(SnippetDto::from).collect())
}

/// HK-001, SNP-005: the "Atalhos" settings section.
#[tauri::command]
pub fn get_shortcut_settings(
    container: State<'_, Container>,
) -> CommandResult<ShortcutSettingsDto> {
    use clipvault_application::ports::SettingsRepository;
    let settings = container.settings.load().map_err(AppError::from)?;
    let snippets = ListSnippets::new(container.snippets.clone()).execute()?;
    Ok(ShortcutSettingsDto {
        open_shortcut: settings.hotkeys.open.to_string(),
        restore_clipboard: settings.snippets.restore_clipboard,
        snippets: snippets.iter().map(SnippetDto::from).collect(),
    })
}

/// SNP-001, SNP-006: creates or edits a snippet and registers its shortcut.
#[tauri::command]
pub fn save_snippet(
    container: State<'_, Container>,
    draft: SnippetDraftDto,
) -> CommandResult<SnippetDto> {
    let saved = SaveSnippet::new(container.snippet_dependencies()).execute(draft.into())?;
    Ok(SnippetDto::from(&saved))
}

/// SNP-006: deletes a snippet and releases its shortcut.
#[tauri::command]
pub fn delete_snippet(container: State<'_, Container>, id: i64) -> CommandResult<()> {
    DeleteSnippet::new(container.snippet_dependencies()).execute(SnippetId::new(id))?;
    Ok(())
}

/// HK-001..004: changes the shortcut that opens the popup; returns it in canonical form.
#[tauri::command]
pub fn change_open_hotkey(
    container: State<'_, Container>,
    shortcut: String,
) -> CommandResult<String> {
    let hotkey = ChangeOpenHotkey::new(container.snippet_dependencies()).execute(&shortcut)?;
    Ok(hotkey.to_string())
}

/// HK-002: releases global shortcuts while the user records a new one.
#[tauri::command]
pub fn set_hotkey_suspension(
    container: State<'_, Container>,
    suspension: SuspensionDto,
) -> CommandResult<()> {
    SuspendHotkeys::new(container.registrar.clone()).execute(suspension.into())?;
    Ok(())
}

/// Whether a setting is turned on, as sent by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ToggleDto {
    On,
    Off,
}

/// SNP-005: restore the previous clipboard content after pasting a snippet.
#[tauri::command]
pub fn set_restore_clipboard(
    container: State<'_, Container>,
    value: ToggleDto,
) -> CommandResult<()> {
    UpdateSettings::new(container.settings.clone(), container.events.clone())
        .execute(|s| s.snippets.restore_clipboard = value == ToggleDto::On)?;
    Ok(())
}

/// CFG-001: opens the settings window. Async on purpose: creating a WebView2 window from a
/// synchronous command (main thread, inside an IPC call) leaves it blank on Windows.
#[tauri::command]
pub async fn open_settings(app: AppHandle<Wry>) {
    open_settings_window(&app, Page::General);
}

/// Opens the settings window on `page` with the current backdrop (tray, popup, command).
pub fn open_settings_window(app: &AppHandle<Wry>, page: Page) {
    popup::hide(app);
    let ui = preferences::settings(app).ui;
    settings_window::open(app, appearance::current(ui).backdrop, page);
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
        appearance: appearance::current(settings.ui),
        locale: preferences::locale(&settings).tag().to_owned(),
    })
}

/// PST-007: closes the popup without pasting.
#[tauri::command]
pub fn hide_popup(app: AppHandle<Wry>) {
    popup::hide(&app);
}
