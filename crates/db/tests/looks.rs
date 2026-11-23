//! Storage of each person's own layout and theme, on both engines.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::entity::user_preference;
use organizr_db::{Database, StoredLook};
use organizr_domain::{GroupLevel, NewGroup, NewUser, PasswordHash, Timestamp, User};
use organizr_testkit::engines::on_each_engine;
use sea_orm::EntityTrait;

/// A user in a group of its own.
async fn insert_user(database: &Database, username: &str) -> User {
    let level = u16::try_from(database.users().count().await.expect("the count works"))
        .expect("the tests make few users");
    let group = database
        .groups()
        .insert(NewGroup {
            level: GroupLevel::new(level).expect("the level is valid"),
            name: format!("Group of {username}"),
            image: None,
            is_default: false,
        })
        .await
        .expect("the group can be stored");
    database
        .users()
        .insert(NewUser {
            username: username.to_owned(),
            email: None,
            password_hash: PasswordHash::new("$2y$10$not.a.real.hash".to_owned()),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored")
}

fn full_look() -> StoredLook {
    StoredLook {
        layout: Some("modern".to_owned()),
        og_theme: Some("Organizr".to_owned()),
        og_style: Some("dark".to_owned()),
        modern_theme: Some("aurora".to_owned()),
        modern_colour_set: Some("ocean".to_owned()),
    }
}

#[tokio::test]
async fn look_is_stored_and_read_back() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin").await;

        database
            .looks()
            .set(user.id, &full_look())
            .await
            .expect("the look can be stored");

        let read = database.looks().get(user.id).await.expect("it reads");
        assert_eq!(read, Some(full_look()));
    })
    .await;
}

#[tokio::test]
async fn look_set_again_replaces_it() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin").await;
        database
            .looks()
            .set(user.id, &full_look())
            .await
            .expect("the look can be stored");
        let second = StoredLook {
            layout: Some("og".to_owned()),
            ..StoredLook::default()
        };

        database
            .looks()
            .set(user.id, &second)
            .await
            .expect("the look can be replaced");

        let read = database.looks().get(user.id).await.expect("it reads");
        assert_eq!(read, Some(second));
    })
    .await;
}

#[tokio::test]
async fn look_cleared_reads_none() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin").await;
        database
            .looks()
            .set(user.id, &full_look())
            .await
            .expect("the look can be stored");

        database
            .looks()
            .clear(user.id)
            .await
            .expect("the look can be cleared");

        assert_eq!(database.looks().get(user.id).await.expect("it reads"), None);
    })
    .await;
}

#[tokio::test]
async fn look_of_a_user_without_one_is_none() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin").await;

        assert_eq!(database.looks().get(user.id).await.expect("it reads"), None);
        database
            .looks()
            .clear(user.id)
            .await
            .expect("clearing nothing is fine");
    })
    .await;
}

#[tokio::test]
async fn look_is_deleted_with_its_user() {
    on_each_engine(async |database| {
        let user = insert_user(database, "robin").await;
        database
            .looks()
            .set(user.id, &full_look())
            .await
            .expect("the look can be stored");

        database
            .users()
            .delete(user.id)
            .await
            .expect("the user can be deleted");

        assert_eq!(database.looks().get(user.id).await.expect("it reads"), None);
        let rows = user_preference::Entity::find()
            .all(database.connection())
            .await
            .expect("the rows read");
        assert_eq!(rows.len(), 0);
    })
    .await;
}
