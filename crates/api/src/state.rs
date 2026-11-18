//! What every request handler can reach. It is built once, at start-up.

use organizr_db::Database;

/// The shared state of the application.
#[derive(Debug, Clone)]
pub struct AppState {
    database: Database,
}

impl AppState {
    /// Creates the state around an open, migrated database.
    pub const fn new(database: Database) -> Self {
        Self { database }
    }

    /// The application database.
    pub const fn database(&self) -> &Database {
        &self.database
    }
}
