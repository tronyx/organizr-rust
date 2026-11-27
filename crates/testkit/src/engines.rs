//! Runs a test once per database engine, each time on a fresh, migrated database.
//!
//! - `SQLite`: always, on a file in a temporary directory.
//! - `PostgreSQL`: when `ORGANIZR_TEST_POSTGRES_URL` holds the URL of a server the test may
//!   create databases on, for example `postgres://postgres:secret@localhost:5432/postgres`.
//!   CI sets it. Each test gets a database of its own.

use std::sync::atomic::{AtomicU32, Ordering};

use organizr_db::{Database, Engine};
use sea_orm::ConnectionTrait;

/// The environment variable with the URL of the `PostgreSQL` test server.
pub const POSTGRES_URL_VARIABLE: &str = "ORGANIZR_TEST_POSTGRES_URL";

/// The URL of the `PostgreSQL` test server, when one is configured.
pub fn postgres_url() -> Option<String> {
    std::env::var(POSTGRES_URL_VARIABLE)
        .ok()
        .filter(|url| !url.is_empty())
}

/// Runs `test` on a fresh database of each available engine.
pub async fn on_each_engine<F>(test: F)
where
    F: AsyncFn(&Database),
{
    on_sqlite(&test).await;
    if let Some(server_url) = postgres_url() {
        on_postgres(&server_url, &test).await;
    }
}

async fn on_sqlite<F>(test: &F)
where
    F: AsyncFn(&Database),
{
    let directory = tempfile::tempdir().expect("a temporary directory can be created");
    let file = directory.path().join("organizr.db");
    let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));

    let database = Database::connect(&url)
        .await
        .expect("the SQLite database opens");
    assert_eq!(database.engine(), Engine::Sqlite);
    database
        .migrate()
        .await
        .expect("the migrations apply on SQLite");

    test(&database).await;

    database.close().await.expect("the SQLite database closes");
}

async fn on_postgres<F>(server_url: &str, test: &F)
where
    F: AsyncFn(&Database),
{
    let fresh = create_postgres(server_url).await;
    fresh
        .database
        .migrate()
        .await
        .expect("the migrations apply on PostgreSQL");

    test(&fresh.database).await;

    fresh.remove().await;
}

/// A database on the `PostgreSQL` test server that no one has migrated: no tables.
#[derive(Debug)]
pub struct FreshPostgres {
    /// The database, open.
    pub database: Database,
    /// The URL of the database, with its password.
    pub url: String,
    server: sea_orm::DatabaseConnection,
    name: String,
}

impl FreshPostgres {
    /// Closes the database and drops it from the server.
    pub async fn remove(self) {
        self.database
            .close()
            .await
            .expect("the PostgreSQL test database closes");
        self.server
            .execute_unprepared(&format!("DROP DATABASE {} WITH (FORCE)", self.name))
            .await
            .expect("the test database can be dropped");
        self.server
            .close()
            .await
            .expect("the connection to the test server closes");
    }
}

/// Creates an empty database on the `PostgreSQL` test server, or gives `None` when no
/// server is configured. Call [`FreshPostgres::remove`] at the end of the test.
pub async fn fresh_postgres() -> Option<FreshPostgres> {
    let server_url = postgres_url()?;
    Some(create_postgres(&server_url).await)
}

async fn create_postgres(server_url: &str) -> FreshPostgres {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let name = format!(
        "organizr_test_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    let server = sea_orm::Database::connect(server_url)
        .await
        .expect("the PostgreSQL test server accepts connections");
    // A database of an earlier run can be left behind, for example when that run was
    // stopped, and Windows gives process numbers out again. It is dropped first.
    server
        .execute_unprepared(&format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)"))
        .await
        .expect("a left-behind test database can be dropped");
    server
        .execute_unprepared(&format!("CREATE DATABASE {name}"))
        .await
        .expect("the test database can be created");

    let (base, _) = server_url
        .rsplit_once('/')
        .expect("the server URL ends with a database name");
    let url = format!("{base}/{name}");
    let database = Database::connect(&url)
        .await
        .expect("the PostgreSQL test database opens");
    assert_eq!(database.engine(), Engine::Postgres);
    FreshPostgres {
        database,
        url,
        server,
        name,
    }
}
