//! What every request handler can reach. It is built once, at start-up.

use std::sync::Arc;

use organizr_db::Database;
use organizr_settings::Settings;

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
    settings: Arc<Settings>,
}

impl AppState {
    /// Creates the state around an open, migrated database and the settings read from it.
    pub fn new(database: Database, settings: Settings) -> Self {
        Self {
            database,
            settings: Arc::new(settings),
        }
    }

    /// The application database.
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// The settings as they were when the application started.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
}
