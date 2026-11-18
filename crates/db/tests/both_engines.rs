//! The database layer must behave the same on `SQLite` and on `PostgreSQL`.
//!
//! Every test here runs once per engine. See `organizr_testkit::engines` for how the
//! engines are chosen.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::{Database, DbError};
use organizr_domain::{
    Group, GroupId, GroupLevel, NewGroup, NewUser, PasswordHash, Timestamp, UserId,
};
use organizr_testkit::engines::{self, on_each_engine};

/// A group with the given level and name, and nothing else set.
fn new_group(level: u16, name: &str) -> NewGroup {
    NewGroup {
        level: GroupLevel::new(level).expect("the level is valid"),
        name: name.to_owned(),
        image: None,
        is_default: false,
    }
}

/// A user with the given name in the given group.
fn new_user(username: &str, group: GroupId) -> NewUser {
    NewUser {
        username: username.to_owned(),
        email: None,
        password_hash: PasswordHash::new("$2y$10$not.a.real.hash".to_owned()),
        group,
        image: None,
        created_at: Timestamp::from_unix_seconds(1_700_000_000),
    }
}

async fn insert_group(database: &Database, level: u16, name: &str) -> Group {
    database
        .groups()
        .insert(new_group(level, name))
        .await
        .expect("the group can be stored")
}

#[tokio::test]
async fn ci_runs_these_tests_on_postgres_too() {
    if std::env::var_os("CI").is_some() {
        assert!(
            engines::postgres_url().is_some(),
            "CI must set {} so that every test also runs on PostgreSQL",
            engines::POSTGRES_URL_VARIABLE
        );
    }
}

#[tokio::test]
async fn url_of_another_engine_is_rejected() {
    let result = Database::connect("mysql://root@localhost/organizr").await;

    assert!(matches!(result, Err(DbError::UnsupportedUrl)), "{result:?}");
}

#[tokio::test]
async fn migrating_a_second_time_changes_nothing() {
    on_each_engine(async |database| {
        insert_group(database, 0, "Admin").await;

        database.migrate().await.expect("a second run succeeds");

        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");
        assert_eq!(groups.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn group_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let stored = database
            .groups()
            .insert(NewGroup {
                level: GroupLevel::new(4).expect("the level is valid"),
                name: "Üser Group".to_owned(),
                image: Some("plugins/images/groups/user.png".to_owned()),
                is_default: true,
            })
            .await
            .expect("the group can be stored");

        let found = database
            .groups()
            .find_by_level(stored.level)
            .await
            .expect("the group can be read");

        assert_eq!(found.as_ref(), Some(&stored));
        let by_id = database
            .groups()
            .find(stored.id)
            .await
            .expect("the group can be read");
        assert_eq!(by_id.as_ref(), Some(&stored));
        let unknown = database
            .groups()
            .find(GroupId::new(12_345))
            .await
            .expect("the lookup works");
        assert_eq!(unknown, None);
        assert_eq!(stored.level.get(), 4);
        assert_eq!(stored.name, "Üser Group");
        assert_eq!(
            stored.image.as_deref(),
            Some("plugins/images/groups/user.png")
        );
        assert!(stored.is_default);
    })
    .await;
}

#[tokio::test]
async fn unknown_level_finds_no_group() {
    on_each_engine(async |database| {
        let found = database
            .groups()
            .find_by_level(GroupLevel::GUEST)
            .await
            .expect("the lookup works");

        assert_eq!(found, None);
    })
    .await;
}

#[tokio::test]
async fn groups_are_listed_from_most_to_least_access() {
    on_each_engine(async |database| {
        insert_group(database, 999, "Guest").await;
        insert_group(database, 0, "Admin").await;
        insert_group(database, 4, "User").await;

        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");

        let names: Vec<&str> = groups.iter().map(|group| group.name.as_str()).collect();
        assert_eq!(names, ["Admin", "User", "Guest"]);
    })
    .await;
}

#[tokio::test]
async fn two_groups_cannot_share_a_level() {
    on_each_engine(async |database| {
        insert_group(database, 1, "Co-Admin").await;

        let result = database.groups().insert(new_group(1, "Another")).await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn two_groups_cannot_share_a_name_whatever_the_letter_case() {
    on_each_engine(async |database| {
        insert_group(database, 1, "Co-Admin").await;

        let result = database.groups().insert(new_group(2, "CO-ADMIN")).await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn users_are_counted() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let before = database.users().count().await.expect("the count works");

        for name in ["one", "two"] {
            database
                .users()
                .insert(new_user(name, group.id))
                .await
                .expect("the user can be stored");
        }

        let after = database.users().count().await.expect("the count works");
        assert_eq!((before, after), (0, 2));
    })
    .await;
}

#[tokio::test]
async fn user_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        // A date after 2038 does not fit a 32-bit column.
        let created_at = Timestamp::from_unix_seconds(4_102_444_800);

        let stored = database
            .users()
            .insert(NewUser {
                email: Some("zoe@example.com".to_owned()),
                image: Some("https://example.com/zoe.png".to_owned()),
                created_at,
                ..new_user("Zoë", group.id)
            })
            .await
            .expect("the user can be stored");

        assert_eq!(stored.username, "Zoë");
        assert_eq!(stored.email.as_deref(), Some("zoe@example.com"));
        assert_eq!(stored.password_hash.expose(), "$2y$10$not.a.real.hash");
        assert_eq!(stored.group, group.id);
        assert!(!stored.locked, "a new account is not locked");
        assert_eq!(stored.image.as_deref(), Some("https://example.com/zoe.png"));
        assert_eq!(stored.created_at, created_at);

        let found = database
            .users()
            .find_by_username("Zoë")
            .await
            .expect("the user can be read");
        assert_eq!(found.as_ref(), Some(&stored));
        let by_id = database
            .users()
            .find(stored.id)
            .await
            .expect("the user can be read");
        assert_eq!(by_id, Some(stored));
        let unknown = database
            .users()
            .find(UserId::new(12_345))
            .await
            .expect("the lookup works");
        assert_eq!(unknown, None);
    })
    .await;
}

#[tokio::test]
async fn user_is_found_whatever_the_letter_case_of_the_name() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let stored = database
            .users()
            .insert(new_user("Chris", group.id))
            .await
            .expect("the user can be stored");

        for name in ["chris", "CHRIS", "cHrIs"] {
            let found = database
                .users()
                .find_by_username(name)
                .await
                .expect("the lookup works");
            assert_eq!(found.as_ref(), Some(&stored), "looked up as `{name}`");
        }

        let missing = database
            .users()
            .find_by_username("christine")
            .await
            .expect("the lookup works");
        assert_eq!(missing, None);
    })
    .await;
}

#[tokio::test]
async fn user_logs_in_with_the_name_or_the_email_address_in_any_letter_case() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let zoe = database
            .users()
            .insert(NewUser {
                email: Some("Zoe.Öz@Example.com".to_owned()),
                ..new_user("Zoë", group.id)
            })
            .await
            .expect("the user can be stored");
        database
            .users()
            .insert(new_user("without-address", group.id))
            .await
            .expect("the user can be stored");

        for text in [
            "Zoë",
            "zoë",
            "ZOË",
            "zoe.öz@example.com",
            "ZOE.ÖZ@EXAMPLE.COM",
        ] {
            let found = database
                .users()
                .find_by_login(text)
                .await
                .expect("the lookup works");
            assert_eq!(found.as_ref(), Some(&zoe), "logged in as `{text}`");
        }
        for text in ["", " zoë", "zoe", "example.com", "zoe.öz@example.com "] {
            let found = database
                .users()
                .find_by_login(text)
                .await
                .expect("the lookup works");
            assert_eq!(found, None, "logged in as `{text}`");
        }
    })
    .await;
}

#[tokio::test]
async fn login_text_that_fits_two_users_finds_the_older_one() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        // The name of the second user is the address of the first, and both have the
        // same address. The PHP application allows this.
        let first = database
            .users()
            .insert(NewUser {
                email: Some("shared@example.com".to_owned()),
                ..new_user("first", group.id)
            })
            .await
            .expect("the user can be stored");
        database
            .users()
            .insert(NewUser {
                email: Some("shared@example.com".to_owned()),
                ..new_user("Shared@Example.com", group.id)
            })
            .await
            .expect("two users can share an address");

        let found = database
            .users()
            .find_by_login("shared@example.com")
            .await
            .expect("the lookup works");

        assert_eq!(found, Some(first));
    })
    .await;
}

#[tokio::test]
async fn password_hash_of_a_user_can_be_replaced() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let chris = database
            .users()
            .insert(new_user("chris", group.id))
            .await
            .expect("the user can be stored");
        let zoe = database
            .users()
            .insert(new_user("zoe", group.id))
            .await
            .expect("the user can be stored");
        let new_hash = PasswordHash::new("$argon2id$v=19$not.a.real.hash".to_owned());

        let changed = database
            .users()
            .set_password_hash(chris.id, &new_hash)
            .await
            .expect("the update works");
        let unknown = database
            .users()
            .set_password_hash(UserId::new(12_345), &new_hash)
            .await
            .expect("the update works");

        assert!(changed, "the user exists");
        assert!(!unknown, "no such user");
        let stored = |name: &'static str| async move {
            database
                .users()
                .find_by_username(name)
                .await
                .expect("the lookup works")
                .expect("the user exists")
        };
        assert_eq!(stored("chris").await.password_hash, new_hash);
        assert_eq!(stored("zoe").await, zoe, "another user is not touched");
    })
    .await;
}

#[tokio::test]
async fn two_users_cannot_share_a_name_whatever_the_letter_case() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        database
            .users()
            .insert(new_user("chris", group.id))
            .await
            .expect("the first user can be stored");

        let result = database.users().insert(new_user("Chris", group.id)).await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn user_cannot_join_a_group_that_does_not_exist() {
    on_each_engine(async |database| {
        let result = database
            .users()
            .insert(new_user("chris", GroupId::new(12_345)))
            .await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn group_with_users_cannot_be_deleted() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        database
            .users()
            .insert(new_user("chris", group.id))
            .await
            .expect("the user can be stored");

        let result = database.groups().delete(group.id).await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");
        assert_eq!(groups, [group]);
    })
    .await;
}

#[tokio::test]
async fn group_without_users_can_be_deleted_once() {
    on_each_engine(async |database| {
        let group = insert_group(database, 5, "Temporary").await;

        let first = database
            .groups()
            .delete(group.id)
            .await
            .expect("the delete works");
        let second = database
            .groups()
            .delete(group.id)
            .await
            .expect("the delete works");

        assert!(first, "the group existed");
        assert!(!second, "the group is gone");
    })
    .await;
}
