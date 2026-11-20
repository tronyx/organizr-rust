//! Database access for Organizr on SQLite and PostgreSQL: entities, migrations, repositories
//! and the secrets store.
//!
//! The person installing chooses the engine with the database URL. All queries and all
//! migrations are written once and run on both engines.

mod bookmarks;
mod categories;
mod entity;
mod error;
mod groups;
mod import;
mod install;
mod invites;
mod migration;
mod sessions;
mod stored_settings;
mod tabs;
mod users;

use std::time::Duration;

use sea_orm::sqlx::sqlite::SqliteJournalMode;
use sea_orm::{ConnectOptions, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

pub use crate::bookmarks::Bookmarks;
pub use crate::categories::Categories;
pub use crate::error::DbError;
pub use crate::groups::Groups;
pub use crate::import::{Import, ImportData, ImportedUser};
pub use crate::install::{Install, NewInstall};
pub use crate::invites::Invites;
pub use crate::sessions::Sessions;
pub use crate::stored_settings::{StoredSetting, StoredSettings};
pub use crate::tabs::Tabs;
pub use crate::users::{Recovery, SealedTwoFactor, UserLogin, Users};

/// How long a connection waits for another writer on SQLite before it gives up.
const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// A database engine the application can run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// SQLite: one file, no server. The default.
    Sqlite,
    /// PostgreSQL: a separate server.
    Postgres,
}

impl Engine {
    /// The engine a database URL names.
    fn from_url(url: &str) -> Result<Self, DbError> {
        match url.split_once(':').map(|(scheme, _)| scheme) {
            Some("sqlite") => Ok(Self::Sqlite),
            Some("postgres" | "postgresql") => Ok(Self::Postgres),
            _ => Err(DbError::UnsupportedUrl),
        }
    }
}

/// The application database: a pool of connections to one engine.
#[derive(Debug, Clone)]
pub struct Database {
    connection: DatabaseConnection,
    engine: Engine,
}

impl Database {
    /// Opens the database a URL names.
    ///
    /// - `sqlite:/path/to/organizr.db` opens that file and creates it when it is missing.
    /// - `postgres://user:password@host/database` connects to a `PostgreSQL` server.
    ///
    /// On SQLite the connection uses write-ahead logging, enforces foreign keys and waits
    /// a few seconds for a busy database instead of failing at once.
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let engine = Engine::from_url(url)?;

        let mut options = ConnectOptions::new(url.to_owned());
        options.sqlx_logging(false);
        if engine == Engine::Sqlite {
            options.map_sqlx_sqlite_opts(|sqlite| {
                sqlite
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .foreign_keys(true)
                    .busy_timeout(SQLITE_BUSY_TIMEOUT)
            });
        }

        let connection = sea_orm::Database::connect(options)
            .await
            .map_err(DbError::Engine)?;
        Ok(Self { connection, engine })
    }

    /// The engine behind this database.
    pub const fn engine(&self) -> Engine {
        self.engine
    }

    /// Brings the schema up to date. Migrations that already ran are skipped.
    pub async fn migrate(&self) -> Result<(), DbError> {
        migration::Migrator::up(&self.connection, None)
            .await
            .map_err(DbError::Engine)
    }

    /// Closes all connections.
    pub async fn close(self) -> Result<(), DbError> {
        self.connection.close().await.map_err(DbError::Engine)
    }

    /// The categories.
    pub const fn categories(&self) -> Categories<'_> {
        Categories::new(&self.connection)
    }

    /// The groups.
    pub const fn groups(&self) -> Groups<'_> {
        Groups::new(&self.connection)
    }

    /// The import of another install.
    pub const fn import(&self) -> Import<'_> {
        Import::new(&self.connection, self.engine)
    }

    /// The set-up of a new install.
    pub const fn install(&self) -> Install<'_> {
        Install::new(&self.connection)
    }

    /// The sessions.
    pub const fn sessions(&self) -> Sessions<'_> {
        Sessions::new(&self.connection)
    }

    /// The stored settings.
    pub const fn settings(&self) -> StoredSettings<'_> {
        StoredSettings::new(&self.connection)
    }

    /// The bookmarks of the bookmark plugin, and their categories.
    pub const fn bookmarks(&self) -> Bookmarks<'_> {
        Bookmarks::new(&self.connection)
    }

    /// The invites of the invites plugin.
    pub const fn invites(&self) -> Invites<'_> {
        Invites::new(&self.connection)
    }

    /// The tabs.
    pub const fn tabs(&self) -> Tabs<'_> {
        Tabs::new(&self.connection)
    }

    /// The users.
    pub const fn users(&self) -> Users<'_> {
        Users::new(&self.connection)
    }
}
