//! Wiring of adapters into use cases (pattern: Composition Root — specs/04 §P-03).

use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;

use clipvault_application::capture::{CaptureWorker, Debouncer, OwnWriteRegistry, WorkerMessage};
use clipvault_application::ports::{AppEvent, EventPublisher, StartupManager};
use clipvault_application::use_cases::{
    ApplyHotkeyBindings, CaptureClipboard, CaptureDependencies, ClearOnExit, PasteDependencies,
    PasteSnippetDependencies, PurgeExpired, ReconcileImages, RetentionDependencies,
    SnippetDependencies,
};
use clipvault_infrastructure::clock::{SystemClock, ThreadDelay};
use clipvault_infrastructure::images::FsImageStore;
use clipvault_infrastructure::sqlite::{
    Database, DatabaseError, SqliteHistoryRepository, SqliteSettingsRepository,
    SqliteSnippetRepository,
};
use clipvault_infrastructure::win32::{
    ClipboardListener, Win32Clipboard, Win32FocusTracker, Win32HotkeyRegistrar,
    Win32KeystrokeSender, Win32RunningApps, startup_manager,
};
use tauri::{AppHandle, Emitter, Wry};

use crate::{hotkeys, preferences, tray};

/// Interval of the background retention purge (RET-004).
const PURGE_INTERVAL: Duration = Duration::from_hours(1);

/// Errors while assembling the application.
#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    /// The database could not be opened.
    #[error(transparent)]
    Database(#[from] DatabaseError),
    /// The image folder could not be prepared.
    #[error("image folder error: {0}")]
    Images(#[from] std::io::Error),
}

/// Publishes application events to the webview and the tray (pattern: Observer — specs/04 §P-07).
pub struct TauriEvents {
    app: AppHandle<Wry>,
}

impl EventPublisher for TauriEvents {
    fn publish(&self, event: AppEvent) {
        let name = match event {
            AppEvent::HistoryChanged => "history://changed",
            AppEvent::CaptureStateChanged => {
                tray::refresh(&self.app);
                "capture://changed"
            }
            AppEvent::SettingsChanged => {
                preferences::apply(&self.app);
                "settings://changed"
            }
        };
        if let Err(error) = self.app.emit(name, ()) {
            tracing::warn!(%error, name, "failed to emit event");
        }
    }
}

/// The running clipboard capture: listener thread → channel → worker thread (specs/03 §4).
struct CaptureService {
    listener: ClipboardListener,
    inbox: Sender<WorkerMessage>,
    worker: JoinHandle<()>,
}

impl CaptureService {
    /// Stops notifications first, then lets the worker finish a pending capture (TRAY-005).
    fn stop(self) {
        self.listener.stop();
        let _ = self.inbox.send(WorkerMessage::Shutdown);
        let _ = self.worker.join();
    }
}

/// Long-lived adapters shared by the use cases.
pub struct Container {
    pub history: Arc<SqliteHistoryRepository>,
    pub images: Arc<FsImageStore>,
    pub settings: Arc<SqliteSettingsRepository>,
    pub clock: Arc<SystemClock>,
    pub events: Arc<dyn EventPublisher>,
    pub own_writes: Arc<OwnWriteRegistry>,
    pub clipboard: Arc<Win32Clipboard>,
    pub focus: Arc<Win32FocusTracker>,
    pub keys: Arc<Win32KeystrokeSender>,
    pub snippets: Arc<SqliteSnippetRepository>,
    pub registrar: Arc<Win32HotkeyRegistrar>,
    pub running_apps: Arc<Win32RunningApps>,
    pub startup: Arc<dyn StartupManager>,
    capture: Mutex<Option<CaptureService>>,
}

impl Container {
    /// Opens storage under `data_dir` (`history.db` and `images/`, specs/03 §7).
    pub fn build(data_dir: &Path, app: AppHandle<Wry>) -> Result<Self, StartupError> {
        let opened = Database::open(&data_dir.join("history.db"))?;
        let on_press_app = app.clone();
        let registrar =
            Win32HotkeyRegistrar::start(move |owner| hotkeys::on_press(&on_press_app, owner))?;
        // REL-005: the user notification is shown by the UI from M4 on.
        if let Some(backup) = &opened.recovered_from {
            tracing::error!(backup = %backup.display(), "corrupt database replaced");
        }
        Ok(Self {
            history: Arc::new(SqliteHistoryRepository::new(opened.database.clone())),
            images: Arc::new(FsImageStore::open(data_dir.join("images"))?),
            settings: Arc::new(SqliteSettingsRepository::new(opened.database.clone())),
            snippets: Arc::new(SqliteSnippetRepository::new(opened.database)),
            registrar: Arc::new(registrar),
            running_apps: Arc::new(Win32RunningApps),
            startup: startup_manager(),
            clock: Arc::new(SystemClock),
            events: Arc::new(TauriEvents { app }),
            own_writes: Arc::new(OwnWriteRegistry::default()),
            clipboard: Arc::new(Win32Clipboard::new()),
            focus: Arc::new(Win32FocusTracker),
            keys: Arc::new(Win32KeystrokeSender),
            capture: Mutex::new(None),
        })
    }

    /// Ports for snippets and shortcuts (SNP-*, HK-*).
    pub fn snippet_dependencies(&self) -> SnippetDependencies {
        SnippetDependencies {
            snippets: self.snippets.clone(),
            settings: self.settings.clone(),
            registrar: self.registrar.clone(),
            clock: self.clock.clone(),
        }
    }

    /// Ports for pasting snippets (SNP-003..005, SNP-007).
    pub fn paste_snippet_dependencies(&self) -> PasteSnippetDependencies {
        PasteSnippetDependencies {
            snippets: self.snippets.clone(),
            reader: self.clipboard.clone(),
            writer: self.clipboard.clone(),
            focus: self.focus.clone(),
            keys: self.keys.clone(),
            own_writes: self.own_writes.clone(),
            settings: self.settings.clone(),
            delay: Arc::new(ThreadDelay),
        }
    }

    /// Registers the open shortcut and the snippet shortcuts (HK-001, SNP-003); returns the
    /// owners taken by other applications (HK-004).
    pub fn start_hotkeys(&self) -> Vec<clipvault_domain::hotkey::HotkeyOwner> {
        match ApplyHotkeyBindings::new(self.snippet_dependencies()).execute() {
            Ok(failures) => failures,
            Err(error) => {
                tracing::error!(%error, "shortcuts could not be registered");
                Vec::new()
            }
        }
    }

    /// Ports for pasting and copying items (PST-*, HIST-011).
    pub fn paste_dependencies(&self) -> PasteDependencies {
        PasteDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            writer: self.clipboard.clone(),
            focus: self.focus.clone(),
            keys: self.keys.clone(),
            own_writes: self.own_writes.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
        }
    }

    /// Starts listening to the clipboard and recording changes (CAP-001..012).
    pub fn start_capture(&self) -> std::io::Result<()> {
        let capture = CaptureClipboard::new(CaptureDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            settings: self.settings.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
            own_writes: self.own_writes.clone(),
        });
        let worker = CaptureWorker::new(
            self.clipboard.clone(),
            capture,
            self.clock.clone(),
            Debouncer::DEFAULT_QUIET_PERIOD,
        );
        let (inbox, receiver) = mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("capture-worker".into())
            .spawn(move || worker.run(&receiver))?;
        let listener = match ClipboardListener::start(inbox.clone()) {
            Ok(listener) => listener,
            Err(error) => {
                let _ = inbox.send(WorkerMessage::Shutdown);
                let _ = worker.join();
                return Err(error);
            }
        };
        *self.capture.lock().unwrap_or_else(PoisonError::into_inner) = Some(CaptureService {
            listener,
            inbox,
            worker,
        });
        tracing::info!("clipboard capture started");
        Ok(())
    }

    /// Stops the capture, finishing pending work (TRAY-005), then clears the history if the
    /// user asked for it (RET-006). Safe to call more than once.
    pub fn shutdown(&self) {
        self.registrar.stop();
        let service = self
            .capture
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(service) = service {
            service.stop();
            tracing::info!("clipboard capture stopped");
        }
        match ClearOnExit::new(
            self.history.clone(),
            self.images.clone(),
            self.settings.clone(),
        )
        .execute()
        {
            Ok(0) => {}
            Ok(removed) => tracing::info!(removed, "history cleared on exit"),
            Err(error) => tracing::warn!(%error, "history could not be cleared on exit"),
        }
    }

    /// Ports for retention changes (RET-004, CFG-004).
    pub fn retention_dependencies(&self) -> RetentionDependencies {
        RetentionDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            settings: self.settings.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
        }
    }

    fn purge(&self) -> PurgeExpired {
        PurgeExpired::new(self.retention_dependencies())
    }

    /// Startup maintenance (RET-004, RET-005) followed by an hourly purge on a background thread.
    pub fn start_maintenance(&self) {
        match ReconcileImages::new(self.history.clone(), self.images.clone()).execute() {
            Ok(report) => tracing::info!(?report, "image reconciliation finished"),
            Err(error) => tracing::warn!(%error, "image reconciliation failed"),
        }
        let purge = self.purge();
        let run = move || match purge.execute() {
            Ok(removed) => tracing::info!(removed, "retention purge finished"),
            Err(error) => tracing::warn!(%error, "retention purge failed"),
        };
        run();
        let spawned = std::thread::Builder::new()
            .name("maintenance".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(PURGE_INTERVAL);
                    run();
                }
            });
        if let Err(error) = spawned {
            tracing::warn!(%error, "could not start the hourly purge");
        }
    }
}
