//! The copy of an install's `SQLite` database into an empty `PostgreSQL` database.
//!
//! Each test that needs `PostgreSQL` returns at once when no test server is configured.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::{Path, PathBuf};

use organizr_db::entity::{
    bookmark_category, bookmark_tab, category, chat_message, extension, group, invite,
    legacy_setting, session, setting, tab, user,
};
use organizr_db::{COPIED_TABLES, Database, DbError};
use organizr_testkit::engines::fresh_postgres;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    EntityTrait, QueryOrder, Statement,
};
use sha2::{Digest, Sha256};

const AWKWARD_TEXT: &str = "O'Brien \\ \"quoted\" 🎬";

fn url_of(file: &Path) -> String {
    format!("sqlite:{}", file.display().to_string().replace('\\', "/"))
}

fn session_row(id: i64, user_id: i64) -> session::ActiveModel {
    session::ActiveModel {
        id: Set(id),
        token_hash: Set(format!("{id:064x}")),
        user_id: Set(user_id),
        user_agent: Set(Some("Firefox".to_owned())),
        ip: Set("192.0.2.1".to_owned()),
        created_at: Set(1_700_000_000),
        expires_at: Set(1_800_000_000),
    }
}

/// Puts at least one row in each of the 12 tables. The ids are not 1, 2, 3.
#[allow(
    clippy::too_many_lines,
    reason = "the seed lists one row for each of the 12 tables"
)]
async fn seed(database: &Database) {
    let connection = database.connection();
    group::ActiveModel {
        id: Set(1),
        level: Set(0),
        name: Set("Administrator".to_owned()),
        name_key: Set("administrator".to_owned()),
        image: Set(Some("images/admin.png".to_owned())),
        is_default: Set(false),
    }
    .insert(connection)
    .await
    .expect("a group is stored");
    group::ActiveModel {
        id: Set(4),
        level: Set(999),
        name: Set("Guest".to_owned()),
        name_key: Set("guest".to_owned()),
        image: Set(None),
        is_default: Set(true),
    }
    .insert(connection)
    .await
    .expect("a group is stored");
    user::ActiveModel {
        id: Set(5),
        username: Set(AWKWARD_TEXT.to_owned()),
        username_key: Set(AWKWARD_TEXT.to_lowercase()),
        email: Set(Some("brien@example.com".to_owned())),
        email_key: Set(Some("brien@example.com".to_owned())),
        password_hash: Set("$argon2id$v=19$m=19456,t=2,p=1$c2FsdA$aGFzaA".to_owned()),
        group_id: Set(1),
        locked: Set(false),
        image: Set(None),
        created_at: Set(1_700_000_000),
        auth_service: Set("internal".to_owned()),
        auth_provider: Set(None),
        auth_subject: Set(Some("subject-1".to_owned())),
        two_factor_kind: Set(Some("totp".to_owned())),
        two_factor_secret: Set(Some("sealed-secret".to_owned())),
        plex_token: Set(Some("sealed-plex-token".to_owned())),
        recovery_hash: Set(Some("recovery".to_owned())),
        recovery_expires_at: Set(Some(1_900_000_000)),
    }
    .insert(connection)
    .await
    .expect("a user is stored");
    category::ActiveModel {
        id: Set(3),
        number: Set(10),
        position: Set(1),
        name: Set("Media".to_owned()),
        name_key: Set("media".to_owned()),
        image: Set(None),
        is_default: Set(true),
    }
    .insert(connection)
    .await
    .expect("a category is stored");
    tab::ActiveModel {
        id: Set(7),
        position: Set(1),
        category_id: Set(3),
        name: Set("Plex".to_owned()),
        name_key: Set("plex".to_owned()),
        url: Set(Some("https://plex.example.com".to_owned())),
        url_local: Set(None),
        is_default: Set(true),
        enabled: Set(true),
        least_access_level: Set(999),
        most_access_level: Set(0),
        add_to_admin: Set(false),
        image: Set(Some("plex".to_owned())),
        kind: Set(1),
        splash: Set(false),
        ping: Set(true),
        ping_url: Set(Some("https://plex.example.com/ping".to_owned())),
        timeout: Set(true),
        timeout_ms: Set(Some(5000)),
        preload: Set(false),
    }
    .insert(connection)
    .await
    .expect("a tab is stored");
    setting::ActiveModel {
        name: Set("title".to_owned()),
        value: Set(serde_text(AWKWARD_TEXT)),
    }
    .insert(connection)
    .await
    .expect("a setting is stored");
    setting::ActiveModel {
        name: Set("smtp.password".to_owned()),
        value: Set("\"sealed:v1:AbCdEf==\"".to_owned()),
    }
    .insert(connection)
    .await
    .expect("a sealed setting is stored");
    legacy_setting::ActiveModel {
        name: Set("oldSetting".to_owned()),
        value: Set("sealed:v1:old".to_owned()),
    }
    .insert(connection)
    .await
    .expect("a legacy setting is stored");
    session_row(4, 5)
        .insert(connection)
        .await
        .expect("a session is stored");
    bookmark_category::ActiveModel {
        id: Set(2),
        number: Set(20),
        position: Set(1),
        name: Set("Reading".to_owned()),
        name_key: Set("reading".to_owned()),
        is_default: Set(true),
    }
    .insert(connection)
    .await
    .expect("a bookmark category is stored");
    bookmark_tab::ActiveModel {
        id: Set(6),
        position: Set(1),
        category_number: Set(20),
        name: Set("Blog".to_owned()),
        name_key: Set("blog".to_owned()),
        url: Set("https://blog.example.com".to_owned()),
        enabled: Set(true),
        least_access_level: Set(999),
        image: Set("fontawesome::book".to_owned()),
        background_color: Set("#000000".to_owned()),
        text_color: Set("#ffffff".to_owned()),
    }
    .insert(connection)
    .await
    .expect("a bookmark is stored");
    invite::ActiveModel {
        id: Set(8),
        code: Set("AbC123".to_owned()),
        code_key: Set("abc123".to_owned()),
        created_at: Set(1_700_000_000),
        email: Set("friend@example.com".to_owned()),
        username: Set("friend".to_owned()),
        used_at: Set(Some(1_700_000_100)),
        used_by: Set(Some("friend".to_owned())),
        used_from: Set(Some("192.0.2.9".to_owned())),
        is_valid: Set(false),
        kind: Set("plex".to_owned()),
        invited_by: Set("admin".to_owned()),
    }
    .insert(connection)
    .await
    .expect("an invite is stored");
    extension::ActiveModel {
        id: Set("hello".to_owned()),
        enabled: Set(true),
        sha256: Set("ab".repeat(32)),
        permissions: Set("storage,network".to_owned()),
        least_access_level: Set(1),
        stored_values: Set(format!("{{\"greeting\":\"{AWKWARD_TEXT}\"}}").replace('\\', "\\\\")),
    }
    .insert(connection)
    .await
    .expect("an extension is stored");
    chat_message::ActiveModel {
        id: Set(9),
        username: Set(AWKWARD_TEXT.to_owned()),
        gravatar: Set("https://gravatar.example.com/x".to_owned()),
        uid: Set("uid-1".to_owned()),
        sent_at: Set(1_700_000_000),
        ip: Set("192.0.2.1".to_owned()),
        message: Set(AWKWARD_TEXT.to_owned()),
    }
    .insert(connection)
    .await
    .expect("a chat message is stored");
}

fn serde_text(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A migrated and seeded `SQLite` database in a folder of its own. The writer stays open.
async fn sqlite_source() -> (tempfile::TempDir, PathBuf, Database) {
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let file = folder.path().join("organizr.db");
    let database = Database::connect(&url_of(&file))
        .await
        .expect("the SQLite file opens");
    database.migrate().await.expect("the migrations apply");
    seed(&database).await;
    (folder, file, database)
}

async fn open_read_only(file: &Path) -> Database {
    Database::open_sqlite_read_only(file)
        .await
        .expect("the SQLite file opens for reading")
}

async fn same_rows<E, C>(source: &Database, target: &Database, key: C)
where
    E: EntityTrait,
    E::Model: PartialEq + std::fmt::Debug + Send + Sync,
    C: ColumnTrait,
{
    let from = E::find()
        .order_by_asc(key)
        .all(source.connection())
        .await
        .expect("the rows of SQLite are read");
    let to = E::find()
        .order_by_asc(key)
        .all(target.connection())
        .await
        .expect("the rows of PostgreSQL are read");
    assert!(!from.is_empty(), "the seed has a row in every table");
    assert_eq!(from, to);
}

#[tokio::test]
async fn copy_gives_postgres_every_row_of_every_table() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    let (_folder, file, writer) = sqlite_source().await;
    let source = open_read_only(&file).await;

    let counts = source
        .copy_into(&postgres.database)
        .await
        .expect("the copy succeeds");

    assert_eq!(counts.len(), 12);
    assert_eq!(counts, source.row_counts().await.expect("rows are counted"));
    assert!(counts.iter().all(|entry| entry.rows >= 1));
    let target = &postgres.database;
    same_rows::<group::Entity, _>(&source, target, group::Column::Id).await;
    same_rows::<user::Entity, _>(&source, target, user::Column::Id).await;
    same_rows::<category::Entity, _>(&source, target, category::Column::Id).await;
    same_rows::<tab::Entity, _>(&source, target, tab::Column::Id).await;
    same_rows::<setting::Entity, _>(&source, target, setting::Column::Name).await;
    same_rows::<legacy_setting::Entity, _>(&source, target, legacy_setting::Column::Name).await;
    same_rows::<session::Entity, _>(&source, target, session::Column::Id).await;
    same_rows::<bookmark_category::Entity, _>(&source, target, bookmark_category::Column::Id).await;
    same_rows::<bookmark_tab::Entity, _>(&source, target, bookmark_tab::Column::Id).await;
    same_rows::<invite::Entity, _>(&source, target, invite::Column::Id).await;
    same_rows::<extension::Entity, _>(&source, target, extension::Column::Id).await;
    same_rows::<chat_message::Entity, _>(&source, target, chat_message::Column::Id).await;

    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

#[tokio::test]
async fn copy_keeps_ids_and_new_records_get_the_next_id() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    let (_folder, file, writer) = sqlite_source().await;
    let source = open_read_only(&file).await;
    source
        .copy_into(&postgres.database)
        .await
        .expect("the copy succeeds");

    let connection = postgres.database.connection();
    let new_tab = tab::ActiveModel {
        id: NotSet,
        position: Set(2),
        category_id: Set(3),
        name: Set("Sonarr".to_owned()),
        name_key: Set("sonarr".to_owned()),
        url: Set(None),
        url_local: Set(None),
        is_default: Set(false),
        enabled: Set(true),
        least_access_level: Set(999),
        most_access_level: Set(0),
        add_to_admin: Set(false),
        image: Set(None),
        kind: Set(1),
        splash: Set(false),
        ping: Set(false),
        ping_url: Set(None),
        timeout: Set(false),
        timeout_ms: Set(None),
        preload: Set(false),
    }
    .insert(connection)
    .await
    .expect("a new tab is stored");
    assert_eq!(new_tab.id, 8);

    let new_user = user::ActiveModel {
        id: NotSet,
        username: Set("second".to_owned()),
        username_key: Set("second".to_owned()),
        email: Set(None),
        email_key: Set(None),
        password_hash: Set("hash".to_owned()),
        group_id: Set(1),
        locked: Set(false),
        image: Set(None),
        created_at: Set(1_700_000_000),
        auth_service: Set("internal".to_owned()),
        auth_provider: Set(None),
        auth_subject: Set(None),
        two_factor_kind: Set(None),
        two_factor_secret: Set(None),
        plex_token: Set(None),
        recovery_hash: Set(None),
        recovery_expires_at: Set(None),
    }
    .insert(connection)
    .await
    .expect("a new user is stored");
    assert_eq!(new_user.id, 6);

    let new_session = session::ActiveModel {
        id: NotSet,
        ..session_row(0, 5)
    };
    let new_session = session::ActiveModel {
        token_hash: Set("f".repeat(64)),
        ..new_session
    }
    .insert(connection)
    .await
    .expect("a new session is stored");
    assert_eq!(new_session.id, 5);

    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

#[tokio::test]
async fn copy_refuses_a_target_with_records_and_changes_nothing() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    postgres
        .database
        .migrate()
        .await
        .expect("the schema is made");
    group::ActiveModel {
        id: Set(1),
        level: Set(0),
        name: Set("Existing".to_owned()),
        name_key: Set("existing".to_owned()),
        image: Set(None),
        is_default: Set(false),
    }
    .insert(postgres.database.connection())
    .await
    .expect("a group is stored");
    let (_folder, file, writer) = sqlite_source().await;
    let source = open_read_only(&file).await;

    let result = source.copy_into(&postgres.database).await;

    assert!(matches!(result, Err(DbError::NotEmpty)), "{result:?}");
    let groups = group::Entity::find()
        .all(postgres.database.connection())
        .await
        .expect("the groups are read");
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups.first().map(|row| row.name.as_str()),
        Some("Existing")
    );
    let counts = postgres
        .database
        .row_counts()
        .await
        .expect("rows are counted");
    assert_eq!(counts.iter().map(|entry| entry.rows).sum::<u64>(), 1);

    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

#[tokio::test]
async fn copy_accepts_a_migrated_empty_target() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    postgres
        .database
        .migrate()
        .await
        .expect("the schema is made");
    let (_folder, file, writer) = sqlite_source().await;
    let source = open_read_only(&file).await;

    let counts = source
        .copy_into(&postgres.database)
        .await
        .expect("the copy succeeds");

    assert_eq!(counts.len(), 12);
    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

fn digest_of(file: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(file).ok()?;
    Some(Sha256::digest(&bytes).to_vec())
}

#[tokio::test]
async fn copy_leaves_the_sqlite_file_unchanged() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    let (folder, file, writer) = sqlite_source().await;
    let wal = folder.path().join("organizr.db-wal");
    let before = (digest_of(&file), digest_of(&wal));
    assert!(
        before.1.is_some(),
        "the writer keeps its rows in the WAL file"
    );

    let source = open_read_only(&file).await;
    assert!(
        !source
            .has_pending_migrations()
            .await
            .expect("pending are read")
    );
    source
        .copy_into(&postgres.database)
        .await
        .expect("the copy succeeds");
    source.close().await.expect("the file closes");

    assert_eq!((digest_of(&file), digest_of(&wal)), before);
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

#[tokio::test]
async fn copy_includes_rows_that_are_still_in_the_wal_file() {
    let (_folder, file, writer) = sqlite_source().await;
    assert_eq!(
        writer
            .journal_mode()
            .await
            .expect("the mode is read")
            .as_deref(),
        Some("wal")
    );

    // The writer is still open, so nothing was moved from the WAL file into the file.
    let source = open_read_only(&file).await;
    let counts = source.row_counts().await.expect("rows are counted");

    assert!(counts.iter().all(|entry| entry.rows >= 1), "{counts:?}");
    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
}

#[tokio::test]
async fn copy_splits_a_large_table_into_several_inserts() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    let (_folder, file, writer) = sqlite_source().await;
    let rows: Vec<session::ActiveModel> = (100..3100).map(|id| session_row(id, 5)).collect();
    session::Entity::insert_many(rows)
        .exec(writer.connection())
        .await
        .expect("the sessions are stored");
    let source = open_read_only(&file).await;

    let counts = source
        .copy_into(&postgres.database)
        .await
        .expect("the copy succeeds");

    let sessions = counts.iter().find(|entry| entry.table == "sessions");
    assert_eq!(sessions.map(|entry| entry.rows), Some(3001));
    same_rows::<session::Entity, _>(&source, &postgres.database, session::Column::Id).await;
    source.close().await.expect("the file closes");
    writer.close().await.expect("the writer closes");
    postgres.remove().await;
}

#[tokio::test]
async fn copy_refuses_the_wrong_engines() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    postgres
        .database
        .migrate()
        .await
        .expect("the schema is made");
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let sqlite = Database::connect(&url_of(&folder.path().join("organizr.db")))
        .await
        .expect("the SQLite file opens");

    let result = postgres.database.copy_into(&sqlite).await;

    assert!(matches!(result, Err(DbError::WrongEngines)), "{result:?}");
    sqlite.close().await.expect("the file closes");
    postgres.remove().await;
}

async fn table_names(database: &Database, query: &str) -> Vec<String> {
    let connection = database.connection();
    let mut names: Vec<String> = connection
        .query_all_raw(Statement::from_string(
            connection.get_database_backend(),
            query,
        ))
        .await
        .expect("the tables are listed")
        .iter()
        .map(|row| {
            row.try_get_by::<String, _>("name")
                .expect("a table has a name")
        })
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn every_table_of_the_schema_is_copied() {
    let mut expected: Vec<String> = COPIED_TABLES
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    expected.sort();

    let (_folder, _file, writer) = sqlite_source().await;
    let in_sqlite = table_names(
        &writer,
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' AND name <> 'seaql_migrations'",
    )
    .await;
    assert_eq!(in_sqlite, expected, "a table of SQLite is not copied");
    writer.close().await.expect("the writer closes");

    if let Some(postgres) = fresh_postgres().await {
        postgres
            .database
            .migrate()
            .await
            .expect("the schema is made");
        let in_postgres = table_names(
            &postgres.database,
            "SELECT table_name AS name FROM information_schema.tables \
             WHERE table_schema = current_schema() AND table_type = 'BASE TABLE' \
             AND table_name <> 'seaql_migrations'",
        )
        .await;
        assert_eq!(in_postgres, expected, "a table of PostgreSQL is not copied");
        postgres.remove().await;
    }
}

#[tokio::test]
async fn old_file_has_pending_migrations() {
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let old = Database::connect(&url_of(&folder.path().join("old.db")))
        .await
        .expect("the SQLite file opens");
    old.migrate_steps(11).await.expect("11 migrations apply");
    assert!(
        old.has_pending_migrations()
            .await
            .expect("pending are read")
    );
    old.migrate().await.expect("the last migration applies");
    assert!(
        !old.has_pending_migrations()
            .await
            .expect("pending are read")
    );
    old.close().await.expect("the file closes");
}

#[tokio::test]
async fn holds_records_is_false_until_a_table_has_a_row() {
    let Some(postgres) = fresh_postgres().await else {
        return;
    };
    let database = &postgres.database;

    assert!(!database.holds_records().await.expect("records are checked"));
    database.migrate().await.expect("the schema is made");
    assert!(!database.holds_records().await.expect("records are checked"));
    group::ActiveModel {
        id: Set(1),
        level: Set(0),
        name: Set("Administrator".to_owned()),
        name_key: Set("administrator".to_owned()),
        image: Set(None),
        is_default: Set(false),
    }
    .insert(database.connection())
    .await
    .expect("a group is stored");
    assert!(database.holds_records().await.expect("records are checked"));

    postgres.remove().await;
}
