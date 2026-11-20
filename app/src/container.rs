//! Wiring of adapters into use cases (pattern: Composition Root — specs/04 §P-03).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use clipvault_application::ports::{AppEvent, EventPublisher};
use clipvault_application::use_cases::{PurgeExpired, ReconcileImages, RetentionDependencies};
use clipvault_infrastructure::clock::SystemClock;
use clipvault_infrastructure::images::FsImageStore;
use clipvault_infrastructure::sqlite::{
    Database, DatabaseError, SqliteHistoryRepository, SqliteSettingsRepository,
};
use tauri::{AppHandle, Emitter, Runtime};

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

/// Publishes application events to the webview (pattern: Observer — specs/04 §P-07).
pub struct TauriEvents<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> EventPublisher for TauriEvents<R> {
    fn publish(&self, event: AppEvent) {
        let name = match event {
            AppEvent::HistoryChanged => "history://changed",
        };
        if let Err(error) = self.app.emit(name, ()) {
            tracing::warn!(%error, name, "failed to emit event");
        }
    }
}

/// Long-lived adapters shared by the use cases.
pub struct Container {
    pub history: Arc<SqliteHistoryRepository>,
    pub images: Arc<FsImageStore>,
    pub settings: Arc<SqliteSettingsRepository>,
    pub clock: Arc<SystemClock>,
    pub events: Arc<dyn EventPublisher>,
}

impl Container {
    /// Opens storage under `data_dir` (`history.db` and `images/`, specs/03 §7).
    pub fn build<R: Runtime>(data_dir: &Path, app: AppHandle<R>) -> Result<Self, StartupError> {
        let opened = Database::open(&data_dir.join("history.db"))?;
        // REL-005: the user notification is shown by the UI from M4 on.
        if let Some(backup) = &opened.recovered_from {
            tracing::error!(backup = %backup.display(), "corrupt database replaced");
        }
        Ok(Self {
            history: Arc::new(SqliteHistoryRepository::new(opened.database.clone())),
            images: Arc::new(FsImageStore::open(data_dir.join("images"))?),
            settings: Arc::new(SqliteSettingsRepository::new(opened.database)),
            clock: Arc::new(SystemClock),
            events: Arc::new(TauriEvents { app }),
        })
    }

    fn purge(&self) -> PurgeExpired {
        PurgeExpired::new(RetentionDependencies {
            history: self.history.clone(),
            images: self.images.clone(),
            settings: self.settings.clone(),
            clock: self.clock.clone(),
            events: self.events.clone(),
        })
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
