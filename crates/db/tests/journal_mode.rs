//! The journal mode of a database on SQLite: write-ahead logging for a new file, the
//! mode that an Admin chose for an existing one, and the mode of a copy for a backup.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::Path;

use organizr_db::{Database, Engine};
use organizr_testkit::engines::on_each_engine;

fn url_of(file: &Path) -> String {
    format!("sqlite:{}", file.display().to_string().replace('\\', "/"))
}

async fn mode(database: &Database) -> Option<String> {
    database
        .journal_mode()
        .await
        .expect("the journal mode can be read")
}

#[tokio::test]
async fn new_file_gets_write_ahead_logging_and_keeps_a_chosen_mode() {
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let file = folder.path().join("organizr.db");

    let database = Database::connect(&url_of(&file))
        .await
        .expect("the database opens");
    database.migrate().await.expect("the migrations apply");
    assert_eq!(mode(&database).await.as_deref(), Some("wal"));

    let now = database
        .set_journal_mode("DELETE")
        .await
        .expect("the journal mode can be set");
    assert_eq!(now.as_deref(), Some("delete"));
    assert_eq!(mode(&database).await.as_deref(), Some("delete"));
    database.close().await.expect("the database closes");

    // The file keeps the mode: it is not set again when the file opens.
    let database = Database::connect(&url_of(&file))
        .await
        .expect("the database opens again");
    assert_eq!(mode(&database).await.as_deref(), Some("delete"));
    let now = database
        .set_journal_mode("WAL")
        .await
        .expect("the journal mode can be set");
    assert_eq!(now.as_deref(), Some("wal"));
    database.close().await.expect("the database closes");

    let database = Database::connect(&url_of(&file))
        .await
        .expect("the database opens again");
    assert_eq!(mode(&database).await.as_deref(), Some("wal"));
    // Another text changes nothing.
    assert_eq!(
        database
            .set_journal_mode("MEMORY")
            .await
            .expect("the answer comes"),
        None
    );
    assert_eq!(mode(&database).await.as_deref(), Some("wal"));
    database.close().await.expect("the database closes");
}

#[tokio::test]
async fn copy_for_a_backup_has_the_mode_of_the_database() {
    for (chosen, expected) in [("WAL", "wal"), ("DELETE", "delete")] {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let database = Database::connect(&url_of(&folder.path().join("organizr.db")))
            .await
            .expect("the database opens");
        database.migrate().await.expect("the migrations apply");
        database
            .set_journal_mode(chosen)
            .await
            .expect("the journal mode can be set");
        let copy = folder.path().join("copy.db");
        assert!(
            database
                .snapshot(&copy)
                .await
                .expect("the copy can be made")
        );
        database.close().await.expect("the database closes");

        let restored = Database::connect(&url_of(&copy))
            .await
            .expect("the copy opens");
        assert_eq!(mode(&restored).await.as_deref(), Some(expected), "{chosen}");
        restored.close().await.expect("the copy closes");
    }
}

#[tokio::test]
async fn postgres_has_no_journal_mode() {
    on_each_engine(async |database| {
        if database.engine() != Engine::Postgres {
            return;
        }
        assert_eq!(mode(database).await, None);
        assert_eq!(
            database
                .set_journal_mode("WAL")
                .await
                .expect("the answer comes"),
            None
        );
    })
    .await;
}

#[tokio::test]
async fn database_on_sqlite_names_its_file() {
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let file = folder.path().join("named.db");
    let database = Database::connect(&url_of(&file))
        .await
        .expect("the database opens");
    let named = database.file().await.expect("the file can be read");
    assert_eq!(
        named.and_then(|named| named.canonicalize().ok()),
        file.canonicalize().ok()
    );
    database.close().await.expect("the database closes");

    on_each_engine(async |database| {
        let named = database.file().await.expect("the file can be read");
        assert_eq!(named.is_some(), database.engine() == Engine::Sqlite);
    })
    .await;
}
