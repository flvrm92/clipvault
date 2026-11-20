//! Tauri commands of the settings window (CFG-001..005). DTO in → use case → DTO out.
// Tauri injects `State`/`AppHandle` by value; that is the framework's calling convention.
#![allow(clippy::needless_pass_by_value)]

use clipvault_application::AppError;
use clipvault_application::ports::SettingsRepository;
use clipvault_application::use_cases::{
    ChangeRetention, ChangeSizeLimit, CompleteWelcome, ExcludeApp, GetStartup, IncludeApp,
    ListExclusionCandidates, RestoreDefaults, SetStartup, SizeLimit, UpdateSettings, WelcomeChoice,
};
use clipvault_domain::retention::RetentionPolicy;
use tauri::{AppHandle, Manager, State, Wry};
use tauri_plugin_opener::OpenerExt;

use crate::commands::ToggleDto;
use crate::container::Container;
use crate::dto::{
    AboutDto, AboutLinkDto, CommandError, LanguageDto, RetentionDto, SettingsDto,
    SettingsSectionDto, SizeLimitDto, StartupStateDto, ThemeModeDto, WelcomeDto,
};
use crate::settings_window::Page;
use crate::{appearance, preferences, welcome_window};

type CommandResult<T> = Result<T, CommandError>;

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn update(
    container: &Container,
    change: impl FnOnce(&mut clipvault_domain::settings::Settings),
) -> CommandResult<()> {
    UpdateSettings::new(container.settings.clone(), container.events.clone()).execute(change)?;
    Ok(())
}

/// CFG-001: the General, History, Privacy and Appearance sections.
#[tauri::command]
pub fn get_settings(container: State<'_, Container>) -> CommandResult<SettingsDto> {
    let settings = container.settings.load().map_err(AppError::from)?;
    Ok(SettingsDto::new(
        &settings,
        preferences::locale(&settings).tag(),
    ))
}

/// I18N-002: changes the interface language; every surface follows without restarting.
#[tauri::command]
pub fn set_language(container: State<'_, Container>, language: LanguageDto) -> CommandResult<()> {
    update(&container, |s| s.ui.language = language.into())
}

/// THM-001: changes the theme.
#[tauri::command]
pub fn set_theme(container: State<'_, Container>, theme: ThemeModeDto) -> CommandResult<()> {
    update(&container, |s| s.ui.theme = theme.into())
}

/// THM-004: uses the Windows accent color instead of the brand color.
#[tauri::command]
pub fn set_use_accent_color(
    container: State<'_, Container>,
    value: ToggleDto,
) -> CommandResult<()> {
    update(&container, |s| {
        s.ui.use_accent_color = value == ToggleDto::On;
    })
}

/// RET-004: how many items a retention policy would remove now.
#[tauri::command]
pub fn preview_retention(
    container: State<'_, Container>,
    retention: RetentionDto,
) -> CommandResult<u32> {
    let policy = RetentionPolicy::try_from(retention)?;
    let removed = ChangeRetention::new(container.retention_dependencies()).preview(policy)?;
    Ok(count(removed))
}

/// RET-001/002/004: saves and applies a retention policy; returns how many items were removed.
#[tauri::command]
pub fn set_retention(
    container: State<'_, Container>,
    retention: RetentionDto,
) -> CommandResult<u32> {
    let policy = RetentionPolicy::try_from(retention)?;
    let removed = ChangeRetention::new(container.retention_dependencies()).execute(policy)?;
    Ok(count(removed))
}

/// RET-006: clears the unpinned history on exit.
#[tauri::command]
pub fn set_clear_on_exit(container: State<'_, Container>, value: ToggleDto) -> CommandResult<()> {
    update(&container, |s| {
        s.retention.clear_on_exit = value == ToggleDto::On;
    })
}

/// CAP-006: changes a size limit to one of the offered options.
#[tauri::command]
pub fn set_size_limit(
    container: State<'_, Container>,
    limit: SizeLimitDto,
    bytes: u64,
) -> CommandResult<()> {
    let limit = match limit {
        SizeLimitDto::Text => SizeLimit::Text,
        SizeLimitDto::Image => SizeLimit::Image,
    };
    ChangeSizeLimit::new(container.settings.clone(), container.events.clone())
        .execute(limit, bytes)?;
    Ok(())
}

/// CAP-009: ignores copies made in an application; returns its normalized name.
#[tauri::command]
pub fn add_excluded_app(container: State<'_, Container>, name: String) -> CommandResult<String> {
    let app =
        ExcludeApp::new(container.settings.clone(), container.events.clone()).execute(&name)?;
    Ok(app.as_str().to_owned())
}

/// CAP-009: captures copies made in an application again.
#[tauri::command]
pub fn remove_excluded_app(container: State<'_, Container>, name: String) -> CommandResult<()> {
    IncludeApp::new(container.settings.clone(), container.events.clone()).execute(&name)?;
    Ok(())
}

/// CAP-009: applications with a visible window that can still be excluded.
#[tauri::command]
pub fn list_running_apps(container: State<'_, Container>) -> CommandResult<Vec<String>> {
    let apps =
        ListExclusionCandidates::new(container.running_apps.clone(), container.settings.clone())
            .execute()?;
    Ok(apps.iter().map(|a| a.as_str().to_owned()).collect())
}

/// CFG-004: restores the defaults of one section; returns how many history items the
/// restored retention removed.
#[tauri::command]
pub fn restore_defaults(
    container: State<'_, Container>,
    section: SettingsSectionDto,
) -> CommandResult<u32> {
    let removed = RestoreDefaults::new(
        container.retention_dependencies(),
        container.snippet_dependencies(),
    )
    .execute(section.into())?;
    Ok(count(removed))
}

/// CFG-005: version shown in "Sobre".
#[tauri::command]
pub fn get_about(app: AppHandle<Wry>) -> AboutDto {
    AboutDto {
        version: app.package_info().version.to_string(),
    }
}

/// CFG-005: opens a fixed link in the default browser.
#[tauri::command]
pub fn open_about_link(app: AppHandle<Wry>, link: AboutLinkDto) {
    if let Err(error) = app.opener().open_url(link.url(), None::<&str>) {
        tracing::warn!(%error, ?link, "link could not be opened");
    }
}

/// CFG-005: opens the data folder in Explorer, for diagnostics.
#[tauri::command]
pub fn open_data_folder(app: AppHandle<Wry>) {
    let folder = app.path().app_local_data_dir();
    let opened = folder.map_err(|e| e.to_string()).and_then(|dir| {
        app.opener()
            .open_path(dir.to_string_lossy(), None::<&str>)
            .map_err(|e| e.to_string())
    });
    if let Err(error) = opened {
        tracing::warn!(%error, "data folder could not be opened");
    }
}

/// Runs a start-with-Windows call off the main thread: the `WinRT` `StartupTask` blocks
/// (ADR-0008).
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| {
            CommandError::from(AppError::Port(
                clipvault_application::ports::PortError::Unavailable(e.to_string()),
            ))
        })?
        .map_err(CommandError::from)
}

/// CFG-003: whether ClipVault starts with Windows.
#[tauri::command]
pub async fn get_startup(app: AppHandle<Wry>) -> CommandResult<StartupStateDto> {
    let manager = app.state::<Container>().startup.clone();
    let state = blocking(move || GetStartup::new(manager).execute()).await?;
    Ok(state.into())
}

/// CFG-003: turns starting with Windows on or off; locked states are returned unchanged.
#[tauri::command]
pub async fn set_startup(app: AppHandle<Wry>, value: ToggleDto) -> CommandResult<StartupStateDto> {
    let manager = app.state::<Container>().startup.clone();
    let state = blocking(move || SetStartup::new(manager).execute(value == ToggleDto::On)).await?;
    Ok(state.into())
}

/// TRAY-006: what the welcome window shows.
#[tauri::command]
pub async fn get_welcome(app: AppHandle<Wry>) -> CommandResult<WelcomeDto> {
    let container = app.state::<Container>();
    let settings = container.settings.load().map_err(AppError::from)?;
    let manager = container.startup.clone();
    let startup = blocking(move || GetStartup::new(manager).execute()).await?;
    Ok(WelcomeDto {
        open_shortcut: settings.hotkeys.open.to_string(),
        startup: startup.into(),
        appearance: appearance::current(settings.ui),
        locale: preferences::locale(&settings).tag().to_owned(),
    })
}

fn welcome(app: &AppHandle<Wry>) -> CompleteWelcome {
    let container = app.state::<Container>();
    CompleteWelcome::new(
        container.settings.clone(),
        container.events.clone(),
        container.startup.clone(),
    )
}

/// TRAY-006: "Começar" / "Abrir configurações"; `null` closes without consent.
#[tauri::command]
pub async fn complete_welcome(
    app: AppHandle<Wry>,
    start_with_windows: Option<bool>,
    open_settings: bool,
) -> CommandResult<StartupStateDto> {
    let choice = start_with_windows.map_or(WelcomeChoice::Dismissed, |start_with_windows| {
        WelcomeChoice::Confirmed { start_with_windows }
    });
    let use_case = welcome(&app);
    let state = blocking(move || use_case.execute(choice)).await;
    welcome_window::close(&app);
    if open_settings {
        crate::commands::open_settings_window(&app, Page::General);
    }
    Ok(state?.into())
}

/// TRAY-006: shows the welcome window on the first run.
pub fn show_welcome_if_pending(app: &AppHandle<Wry>) {
    match welcome(app).is_pending() {
        Ok(true) => welcome_window::open(app),
        Ok(false) => {}
        Err(error) => tracing::warn!(%error, "first run state could not be read"),
    }
}

/// TRAY-006: the welcome window was closed without a choice.
pub fn dismiss_welcome(app: &AppHandle<Wry>) {
    let use_case = welcome(app);
    if !use_case.is_pending().unwrap_or(false) {
        return;
    }
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = use_case.execute(WelcomeChoice::Dismissed) {
            tracing::warn!(%error, "first run could not be recorded");
        }
    });
}
