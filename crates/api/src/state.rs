//! What every request handler can reach. It is built once, at start-up.

use std::sync::Arc;

use organizr_db::Database;
use organizr_settings::Settings;

use crate::client::TrustedProxies;

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
    settings: Arc<Settings>,
    trusted_proxies: Arc<TrustedProxies>,
}

impl AppState {
    /// Creates the state around an open, migrated database and the settings read from it.
    /// No proxy is trusted until [`AppState::with_trusted_proxies`] names some.
    pub fn new(database: Database, settings: Settings) -> Self {
        Self {
            database,
            settings: Arc::new(settings),
            trusted_proxies: Arc::new(TrustedProxies::default()),
        }
    }

    /// Sets the reverse proxies whose forwarding headers are believed.
    #[must_use]
    pub fn with_trusted_proxies(mut self, trusted_proxies: TrustedProxies) -> Self {
        self.trusted_proxies = Arc::new(trusted_proxies);
        self
    }

    /// The application database.
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// The settings as they were when the application started.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The reverse proxies whose forwarding headers are believed.
    pub fn trusted_proxies(&self) -> &TrustedProxies {
        &self.trusted_proxies
    }
}
