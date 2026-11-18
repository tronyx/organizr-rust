//! Runs a database test once per engine, each time on a fresh, migrated database.
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
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let name = format!(
        "organizr_test_{}_{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    let server = sea_orm::Database::connect(server_url)
        .await
        .expect("the PostgreSQL test server accepts connections");
    server
        .execute_unprepared(&format!("CREATE DATABASE {name}"))
        .await
        .expect("the test database can be created");

    let (base, _) = server_url
        .rsplit_once('/')
        .expect("the server URL ends with a database name");
    let database = Database::connect(&format!("{base}/{name}"))
        .await
        .expect("the PostgreSQL test database opens");
    assert_eq!(database.engine(), Engine::Postgres);
    database
        .migrate()
        .await
        .expect("the migrations apply on PostgreSQL");

    test(&database).await;

    database
        .close()
        .await
        .expect("the PostgreSQL test database closes");
    server
        .execute_unprepared(&format!("DROP DATABASE {name} WITH (FORCE)"))
        .await
        .expect("the test database can be dropped");
    server
        .close()
        .await
        .expect("the connection to the test server closes");
}
