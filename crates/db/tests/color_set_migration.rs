//! The migration that gives the stored color set its American name, on both engines: a
//! database at the previous migration keeps its values.
//!
//! The `PostgreSQL` half of each test runs only when a test server is configured.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::Database;
use organizr_db::entity::{group, setting, user};
use organizr_testkit::engines::fresh_postgres;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, QueryOrder};

/// The number of migrations before the one that renames the color set.
const PREVIOUS_VERSION: u32 = 13;

/// Runs `test` on a `SQLite` file and, when configured, on a `PostgreSQL` database, each at
/// the previous migration.
async fn at_previous_version<F>(test: F)
where
    F: AsyncFn(&Database),
{
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let file = folder.path().join("organizr.db");
    let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));
    let sqlite = Database::connect(&url)
        .await
        .expect("the SQLite file opens");
    sqlite
        .migrate_steps(PREVIOUS_VERSION)
        .await
        .expect("the previous migrations apply on SQLite");
    test(&sqlite).await;
    sqlite.close().await.expect("the SQLite file closes");

    if let Some(postgres) = fresh_postgres().await {
        postgres
            .database
            .migrate_steps(PREVIOUS_VERSION)
            .await
            .expect("the previous migrations apply on PostgreSQL");
        test(&postgres.database).await;
        postgres.remove().await;
    }
}

/// A group and a user with the id 5, and the user's look with the old column.
async fn store_old_look(database: &Database) {
    let connection = database.connection();
    group::ActiveModel {
        id: Set(1),
        level: Set(0),
        name: Set("Administrator".to_owned()),
        name_key: Set("administrator".to_owned()),
        image: Set(None),
        is_default: Set(false),
    }
    .insert(connection)
    .await
    .expect("a group is stored");
    user::ActiveModel {
        id: Set(5),
        username: Set("robin".to_owned()),
        username_key: Set("robin".to_owned()),
        email: Set(None),
        email_key: Set(None),
        password_hash: Set("$2y$10$not.a.real.hash".to_owned()),
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
    .expect("a user is stored");
    connection
        .execute_unprepared(
            "INSERT INTO user_preferences (user_id, layout, modern_theme, modern_colour_set) \
             VALUES (5, 'modern', 'silo', 'rose')",
        )
        .await
        .expect("a look is stored in the old column");
}

async fn store_setting(database: &Database, name: &str, value: &str) {
    setting::ActiveModel {
        name: Set(name.to_owned()),
        value: Set(value.to_owned()),
    }
    .insert(database.connection())
    .await
    .expect("a setting is stored");
}

/// Every stored setting, as name and value, by name.
async fn settings(database: &Database) -> Vec<(String, String)> {
    setting::Entity::find()
        .order_by_asc(setting::Column::Name)
        .all(database.connection())
        .await
        .expect("the settings read")
        .into_iter()
        .map(|row| (row.name, row.value))
        .collect()
}

#[tokio::test]
async fn the_stored_color_set_keeps_its_value() {
    at_previous_version(async |database| {
        store_old_look(database).await;
        store_setting(database, "modernColourSet", "\"rose\"").await;
        store_setting(database, "title", "\"Home\"").await;

        database.migrate().await.expect("the migration applies");

        let look = database
            .looks()
            .get(organizr_domain::UserId::new(5))
            .await
            .expect("the look reads")
            .expect("the look is there");
        assert_eq!(look.modern_color_set.as_deref(), Some("rose"));
        assert_eq!(look.modern_theme.as_deref(), Some("silo"));
        assert_eq!(
            settings(database).await,
            [
                ("modernColorSet".to_owned(), "\"rose\"".to_owned()),
                ("title".to_owned(), "\"Home\"".to_owned()),
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn the_new_setting_wins_over_the_old_one() {
    at_previous_version(async |database| {
        store_setting(database, "modernColourSet", "\"rose\"").await;
        store_setting(database, "modernColorSet", "\"ember\"").await;

        database.migrate().await.expect("the migration applies");

        assert_eq!(
            settings(database).await,
            [("modernColorSet".to_owned(), "\"ember\"".to_owned())]
        );
    })
    .await;
}

#[tokio::test]
async fn a_database_without_the_setting_stays_without_it() {
    at_previous_version(async |database| {
        database.migrate().await.expect("the migration applies");

        assert_eq!(settings(database).await, []);
        assert!(
            !database
                .has_pending_migrations()
                .await
                .expect("pending are read")
        );
    })
    .await;
}

#[tokio::test]
async fn only_the_new_setting_stays_as_it_is() {
    at_previous_version(async |database| {
        store_setting(database, "modernColorSet", "\"ember\"").await;

        database.migrate().await.expect("the migration applies");

        assert_eq!(
            settings(database).await,
            [("modernColorSet".to_owned(), "\"ember\"".to_owned())]
        );
    })
    .await;
}

#[tokio::test]
async fn a_failed_step_leaves_the_old_names_and_the_migration_pending() {
    at_previous_version(async |database| {
        store_old_look(database).await;
        // Without the table of the settings, the second step of the migration fails.
        database
            .connection()
            .execute_unprepared("DROP TABLE settings")
            .await
            .expect("the table of the settings is dropped");

        assert!(database.migrate().await.is_err());

        // The first step, the rename of the column, was rolled back with it.
        database
            .connection()
            .execute_unprepared("SELECT modern_colour_set FROM user_preferences")
            .await
            .expect("the column keeps its old name");
        assert!(
            database
                .has_pending_migrations()
                .await
                .expect("pending are read")
        );
    })
    .await;
}

#[tokio::test]
async fn down_gives_back_the_old_names_and_values() {
    at_previous_version(async |database| {
        store_old_look(database).await;
        store_setting(database, "modernColourSet", "\"rose\"").await;
        database.migrate().await.expect("the migration applies");

        database
            .migrate_down_steps(1)
            .await
            .expect("the migration rolls back");

        let row = database
            .connection()
            .query_one_raw(sea_orm::Statement::from_string(
                database.connection().get_database_backend(),
                "SELECT modern_colour_set FROM user_preferences WHERE user_id = 5",
            ))
            .await
            .expect("the old column reads")
            .expect("the look is there");
        assert_eq!(
            row.try_get::<Option<String>>("", "modern_colour_set")
                .expect("the value reads")
                .as_deref(),
            Some("rose")
        );
        assert_eq!(
            settings(database).await,
            [("modernColourSet".to_owned(), "\"rose\"".to_owned())]
        );
    })
    .await;
}
