//! Database access for Organizr on SQLite and PostgreSQL: entities, migrations, repositories
//! and the secrets store.
//!
//! The person installing chooses the engine with the database URL. All queries and all
//! migrations are written once and run on both engines.

mod bookmarks;
mod categories;
mod chat;
mod copy;
#[doc(hidden)]
pub mod entity;
mod error;
mod extensions;
mod groups;
mod import;
mod install;
mod invites;
mod looks;
mod migration;
mod sessions;
mod stored_settings;
mod tabs;
mod users;

use std::time::Duration;

use sea_orm::sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sea_orm::sqlx::{ConnectOptions as _, Connection as _};
use sea_orm::{ConnectOptions, ConnectionTrait, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

pub use crate::bookmarks::Bookmarks;
pub use crate::categories::Categories;
pub use crate::chat::Chat;
pub use crate::copy::{COPIED_TABLES, TableCount};
pub use crate::error::DbError;
pub use crate::extensions::{ApprovedExtension, Extensions};
pub use crate::groups::Groups;
pub use crate::import::{Import, ImportData, ImportedUser};
pub use crate::install::{Install, NewInstall};
pub use crate::invites::Invites;
pub use crate::looks::{Looks, StoredLook};
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
    /// On SQLite the connection enforces foreign keys and waits a few seconds for a busy
    /// database instead of failing at once. A new database file gets write-ahead logging.
    /// The journal mode is kept in the file, so an existing file keeps the mode that it
    /// has, also one that an Admin chose with [`Database::set_journal_mode`].
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let engine = Engine::from_url(url)?;

        let mut options = ConnectOptions::new(url.to_owned());
        options.sqlx_logging(false);
        if engine == Engine::Sqlite {
            options.map_sqlx_sqlite_opts(|sqlite| {
                sqlite
                    .create_if_missing(true)
                    .foreign_keys(true)
                    .busy_timeout(SQLITE_BUSY_TIMEOUT)
            });
        }

        let connection = sea_orm::Database::connect(options)
            .await
            .map_err(DbError::Engine)?;
        if engine == Engine::Sqlite {
            // A new file has no pages yet. Write-ahead logging is then set once, and the
            // file keeps it.
            let pages = connection
                .query_one_raw(sea_orm::Statement::from_string(
                    connection.get_database_backend(),
                    "PRAGMA page_count",
                ))
                .await?
                .and_then(|row| row.try_get_by_index::<i64>(0).ok());
            if pages == Some(0) {
                connection
                    .execute_unprepared("PRAGMA journal_mode = WAL")
                    .await?;
            }
        }
        Ok(Self { connection, engine })
    }

    /// The journal mode of a database on `SQLite`, in lower case as `SQLite` names it, for
    /// example `wal` or `delete`. `None` on `PostgreSQL`, which has no journal mode.
    pub async fn journal_mode(&self) -> Result<Option<String>, DbError> {
        if self.engine != Engine::Sqlite {
            return Ok(None);
        }
        let row = self
            .connection
            .query_one_raw(sea_orm::Statement::from_string(
                self.connection.get_database_backend(),
                "PRAGMA journal_mode",
            ))
            .await?;
        Ok(row.and_then(|row| row.try_get_by_index::<String>(0).ok()))
    }

    /// Sets the journal mode of a database on `SQLite` and returns the mode that it has
    /// then, which `SQLite` may have kept. `None` on `PostgreSQL`. `mode` is `WAL` or
    /// `DELETE`; any other text changes nothing and gives `None`.
    ///
    /// `SQLite` leaves write-ahead logging only while no other connection has the file
    /// open. So the idle connections of the pool are closed first; the pool opens new
    /// ones when it needs them, and they take the mode of the file. A request that uses
    /// a connection at that moment can still keep the mode from changing: the change
    /// then fails after the time that a busy database is waited for.
    pub async fn set_journal_mode(&self, mode: &str) -> Result<Option<String>, DbError> {
        let statement = match mode {
            "WAL" => "PRAGMA journal_mode = WAL",
            "DELETE" => "PRAGMA journal_mode = DELETE",
            _ => return Ok(None),
        };
        if self.engine != Engine::Sqlite {
            return Ok(None);
        }
        let failed =
            |error: sea_orm::sqlx::Error| DbError::Engine(DbErr::Custom(error.to_string()));
        let pool = self.connection.get_sqlite_connection_pool();
        let mut own = pool.acquire().await.map_err(failed)?;
        while let Some(idle) = pool.try_acquire() {
            idle.close().await.map_err(failed)?;
        }
        let now: String = sea_orm::sqlx::query_scalar(statement)
            .fetch_one(&mut *own)
            .await
            .map_err(failed)?;
        Ok(Some(now))
    }

    /// The engine behind this database.
    pub const fn engine(&self) -> Engine {
        self.engine
    }

    /// Writes a copy of the database to a new file, for a backup. The copy is whole
    /// also while other requests write to the database.
    ///
    /// Returns whether a copy was made: only `SQLite` keeps its data in a file. A
    /// database on `PostgreSQL` is saved with the tools of `PostgreSQL`.
    pub async fn snapshot(&self, file: &std::path::Path) -> Result<bool, DbError> {
        if self.engine != Engine::Sqlite {
            return Ok(false);
        }
        // The file name is a text of the statement, so a quote in it is doubled.
        let name = file.display().to_string().replace('\'', "''");
        self.connection
            .execute_unprepared(&format!("VACUUM INTO '{name}'"))
            .await?;
        // The copy of `VACUUM INTO` has the journal mode `delete`. A restored copy keeps
        // the mode of the database it came from.
        if self.journal_mode().await?.as_deref() == Some("wal") {
            let failed =
                |error: sea_orm::sqlx::Error| DbError::Engine(DbErr::Custom(error.to_string()));
            let mut copy = SqliteConnectOptions::new()
                .filename(file)
                .connect()
                .await
                .map_err(failed)?;
            sea_orm::sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&mut copy)
                .await
                .map_err(failed)?;
            copy.close().await.map_err(failed)?;
        }
        Ok(true)
    }

    /// Brings the schema up to date. Migrations that already ran are skipped.
    pub async fn migrate(&self) -> Result<(), DbError> {
        migration::Migrator::up(&self.connection, None)
            .await
            .map_err(DbError::Engine)
    }

    /// Runs only the first `steps` migrations. For tests that need an old schema.
    #[doc(hidden)]
    pub async fn migrate_steps(&self, steps: u32) -> Result<(), DbError> {
        migration::Migrator::up(&self.connection, Some(steps))
            .await
            .map_err(DbError::Engine)
    }

    /// Rolls back the last `steps` migrations. For tests of a migration's `down`.
    #[doc(hidden)]
    pub async fn migrate_down_steps(&self, steps: u32) -> Result<(), DbError> {
        migration::Migrator::down(&self.connection, Some(steps))
            .await
            .map_err(DbError::Engine)
    }

    /// The connection, for tests that read or write rows directly.
    #[doc(hidden)]
    pub const fn connection(&self) -> &DatabaseConnection {
        &self.connection
    }

    /// Opens an existing `SQLite` file for reading only. The file is not created, and
    /// nothing is written to it: no migration, no journal mode, no table. Fails when the
    /// file does not exist.
    pub async fn open_sqlite_read_only(file: &std::path::Path) -> Result<Self, DbError> {
        let options = SqliteConnectOptions::new()
            .filename(file)
            .read_only(true)
            .foreign_keys(true)
            .busy_timeout(SQLITE_BUSY_TIMEOUT);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .map_err(|error| DbError::Engine(DbErr::Custom(error.to_string())))?;
        Ok(Self {
            connection: sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool),
            engine: Engine::Sqlite,
        })
    }

    /// Whether a migration of this version of the application has not run yet on this
    /// database. Changes nothing.
    pub async fn has_pending_migrations(&self) -> Result<bool, DbError> {
        Ok(
            !migration::Migrator::get_pending_migrations_read_only(&self.connection)
                .await?
                .is_empty(),
        )
    }

    /// Closes all connections.
    pub async fn close(self) -> Result<(), DbError> {
        self.connection.close().await.map_err(DbError::Engine)
    }

    /// The categories.
    pub const fn categories(&self) -> Categories<'_> {
        Categories::new(&self.connection)
    }

    /// The messages of the chat plugin.
    pub const fn chat(&self) -> Chat<'_> {
        Chat::new(&self.connection)
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

    /// The layout and theme that each person chose.
    pub const fn looks(&self) -> Looks<'_> {
        Looks::new(&self.connection)
    }

    /// The stored settings.
    pub const fn settings(&self) -> StoredSettings<'_> {
        StoredSettings::new(&self.connection)
    }

    /// The bookmarks of the bookmark plugin, and their categories.
    pub const fn bookmarks(&self) -> Bookmarks<'_> {
        Bookmarks::new(&self.connection)
    }

    /// The extensions that an Admin approved.
    pub const fn extensions(&self) -> Extensions<'_> {
        Extensions::new(&self.connection)
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
