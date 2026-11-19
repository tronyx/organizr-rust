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
    Change, Group, GroupChanges, GroupId, GroupLevel, NewGroup, NewUser, PasswordHash, Timestamp,
    User, UserChanges, UserId,
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
async fn name_or_address_of_a_user_is_taken_whatever_the_letter_case() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        database
            .users()
            .insert(NewUser {
                email: Some("Zoe@Example.com".to_owned()),
                ..new_user("Zoë", group.id)
            })
            .await
            .expect("the user can be stored");
        database
            .users()
            .insert(new_user("without-address", group.id))
            .await
            .expect("the user can be stored");
        let is_taken = async |username: &str, email: &str| {
            database
                .users()
                .is_taken(username, email)
                .await
                .expect("the question can be answered")
        };

        assert!(is_taken("zoë", "new@example.com").await, "the name");
        assert!(is_taken("ZOË", "new@example.com").await, "the name");
        assert!(is_taken("new", "zoe@example.com").await, "the address");
        assert!(is_taken("new", "ZOE@EXAMPLE.COM").await, "the address");
        assert!(is_taken("without-address", "new@example.com").await);
        assert!(!is_taken("new", "new@example.com").await);
        // A name is not compared with an address, and an address not with a name.
        assert!(!is_taken("zoe@example.com", "new@example.com").await);
        assert!(!is_taken("new", "zoë").await);
        // A user without an address does not take the empty address.
        assert!(!is_taken("new", "").await);
    })
    .await;
}

#[tokio::test]
async fn default_group_is_the_group_that_new_users_join() {
    on_each_engine(async |database| {
        let none = database
            .groups()
            .find_default()
            .await
            .expect("the lookup works");
        assert_eq!(none, None);

        insert_group(database, 0, "Admin").await;
        let user = database
            .groups()
            .insert(NewGroup {
                is_default: true,
                ..new_group(4, "User")
            })
            .await
            .expect("the group can be stored");
        let found = database
            .groups()
            .find_default()
            .await
            .expect("the lookup works");
        assert_eq!(found.as_ref(), Some(&user));

        // A new default group takes the mark from the group that had it.
        let super_user = database
            .groups()
            .insert(NewGroup {
                is_default: true,
                ..new_group(2, "Super User")
            })
            .await
            .expect("the group can be stored");
        let found = database
            .groups()
            .find_default()
            .await
            .expect("the lookup works");
        assert_eq!(found, Some(super_user));
        let user = database
            .groups()
            .find(user.id)
            .await
            .expect("the lookup works")
            .expect("the group exists");
        assert!(!user.is_default);
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

/// Changes that keep every field.
fn no_changes() -> GroupChanges {
    GroupChanges {
        name: Change::Keep,
        image: Change::Keep,
        is_default: Change::Keep,
    }
}

#[tokio::test]
async fn group_is_changed_field_by_field() {
    on_each_engine(async |database| {
        let editors = insert_group(database, 5, "Editors").await;
        let groups = database.groups();

        let unchanged = groups
            .update(editors.id, no_changes())
            .await
            .expect("the update works");
        assert_eq!(unchanged.as_ref(), Some(&editors));

        let changed = groups
            .update(
                editors.id,
                GroupChanges {
                    name: Change::Set("Writers".to_owned()),
                    image: Change::Set("writers.png".to_owned()),
                    is_default: Change::Set(true),
                },
            )
            .await
            .expect("the update works")
            .expect("the group exists");
        assert_eq!(
            changed,
            Group {
                name: "Writers".to_owned(),
                image: Some("writers.png".to_owned()),
                is_default: true,
                ..editors.clone()
            }
        );
        let stored = groups.find(editors.id).await.expect("the lookup works");
        assert_eq!(stored, Some(changed));
        // The old name is free, and the new one is taken in any letter case.
        assert!(
            !groups
                .is_name_taken("editors", None)
                .await
                .expect("the lookup works")
        );
        assert!(
            groups
                .is_name_taken("WRITERS", None)
                .await
                .expect("the lookup works")
        );

        let missing = groups
            .update(GroupId::new(editors.id.get() + 100), no_changes())
            .await
            .expect("the update works");
        assert_eq!(missing, None);
    })
    .await;
}

#[tokio::test]
async fn group_that_becomes_the_default_takes_the_mark_from_the_others() {
    on_each_engine(async |database| {
        let groups = database.groups();
        let user = groups
            .insert(NewGroup {
                is_default: true,
                ..new_group(4, "User")
            })
            .await
            .expect("the group can be stored");
        let editors = insert_group(database, 5, "Editors").await;

        // A change that keeps the mark off takes it from nobody.
        groups
            .update(
                editors.id,
                GroupChanges {
                    is_default: Change::Set(false),
                    ..no_changes()
                },
            )
            .await
            .expect("the update works");
        assert_eq!(
            groups.find_default().await.expect("the lookup works"),
            Some(user.clone())
        );

        groups
            .update(
                editors.id,
                GroupChanges {
                    is_default: Change::Set(true),
                    ..no_changes()
                },
            )
            .await
            .expect("the update works");
        let default = groups.find_default().await.expect("the lookup works");
        assert_eq!(default.map(|group| group.id), Some(editors.id));
        let user = groups
            .find(user.id)
            .await
            .expect("the lookup works")
            .expect("the group exists");
        assert!(!user.is_default);
    })
    .await;
}

#[tokio::test]
async fn changed_name_of_a_group_cannot_be_the_name_of_another_group() {
    on_each_engine(async |database| {
        insert_group(database, 4, "User").await;
        let editors = insert_group(database, 5, "Editors").await;
        let groups = database.groups();

        let result = groups
            .update(
                editors.id,
                GroupChanges {
                    name: Change::Set("USER".to_owned()),
                    ..no_changes()
                },
            )
            .await;
        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");

        // A group can take its own name in another letter case.
        let renamed = groups
            .update(
                editors.id,
                GroupChanges {
                    name: Change::Set("EDITORS".to_owned()),
                    ..no_changes()
                },
            )
            .await
            .expect("the update works")
            .expect("the group exists");
        assert_eq!(renamed.name, "EDITORS");
        assert!(
            groups
                .is_name_taken("user", Some(editors.id))
                .await
                .expect("the lookup works")
        );
        assert!(
            !groups
                .is_name_taken("editors", Some(editors.id))
                .await
                .expect("the lookup works")
        );
        assert!(
            groups
                .is_name_taken("editors", None)
                .await
                .expect("the lookup works")
        );
        assert!(
            !groups
                .is_name_taken("nobody", None)
                .await
                .expect("the lookup works")
        );
    })
    .await;
}

#[tokio::test]
async fn highest_level_below_guest_leaves_guest_out() {
    on_each_engine(async |database| {
        let groups = database.groups();
        assert_eq!(
            groups
                .highest_level_below_guest()
                .await
                .expect("the lookup works"),
            None
        );

        insert_group(database, 999, "Guest").await;
        assert_eq!(
            groups
                .highest_level_below_guest()
                .await
                .expect("the lookup works"),
            None
        );

        insert_group(database, 0, "Admin").await;
        insert_group(database, 7, "Editors").await;
        insert_group(database, 4, "User").await;
        assert_eq!(
            groups
                .highest_level_below_guest()
                .await
                .expect("the lookup works"),
            Some(GroupLevel::new(7).expect("the level is valid"))
        );
    })
    .await;
}

#[tokio::test]
async fn users_of_a_group_are_counted() {
    on_each_engine(async |database| {
        let user = insert_group(database, 4, "User").await;
        let editors = insert_group(database, 5, "Editors").await;
        for name in ["zoe", "yan"] {
            database
                .users()
                .insert(new_user(name, user.id))
                .await
                .expect("the user can be stored");
        }

        let groups = database.groups();
        assert_eq!(
            groups.user_count(user.id).await.expect("the count works"),
            2
        );
        assert_eq!(
            groups
                .user_count(editors.id)
                .await
                .expect("the count works"),
            0
        );
    })
    .await;
}

#[tokio::test]
async fn user_is_changed_field_by_field() {
    on_each_engine(async |database| {
        let user_group = insert_group(database, 4, "User").await;
        let editors = insert_group(database, 5, "Editors").await;
        let users = database.users();
        let zoe = users
            .insert(new_user("zoe", user_group.id))
            .await
            .expect("the user can be stored");

        let unchanged = users
            .update(zoe.id, UserChanges::none())
            .await
            .expect("the update works");
        assert_eq!(unchanged.as_ref(), Some(&zoe));

        let new_hash = PasswordHash::new("$argon2id$another.hash".to_owned());
        let changed = users
            .update(
                zoe.id,
                UserChanges {
                    username: Change::Set("Zoë".to_owned()),
                    email: Change::Set("Zoe@Example.com".to_owned()),
                    password_hash: Change::Set(new_hash.clone()),
                    group: Change::Set(editors.id),
                    locked: Change::Set(true),
                    image: Change::Set(Some("zoe.png".to_owned())),
                },
            )
            .await
            .expect("the update works")
            .expect("the user exists");
        assert_eq!(
            changed,
            User {
                username: "Zoë".to_owned(),
                email: Some("Zoe@Example.com".to_owned()),
                password_hash: new_hash,
                group: editors.id,
                locked: true,
                image: Some("zoe.png".to_owned()),
                ..zoe.clone()
            }
        );
        assert_eq!(
            users.find(zoe.id).await.expect("the lookup works"),
            Some(changed.clone())
        );
        // The user is found by the new name and the new address, in any letter case.
        for login in ["ZOË", "zoe@example.com"] {
            let found = users.find_by_login(login).await.expect("the lookup works");
            assert_eq!(found.as_ref(), Some(&changed), "{login}");
        }
        assert_eq!(
            users
                .find_by_username("zoe")
                .await
                .expect("the lookup works"),
            None
        );

        // A picture can be taken away, and a lock can be opened.
        let plain = users
            .update(
                zoe.id,
                UserChanges {
                    locked: Change::Set(false),
                    image: Change::Set(None),
                    ..UserChanges::none()
                },
            )
            .await
            .expect("the update works")
            .expect("the user exists");
        assert!(!plain.locked);
        assert_eq!(plain.image, None);
        assert_eq!(plain.username, "Zoë");

        let missing = users
            .update(UserId::new(zoe.id.get() + 100), UserChanges::none())
            .await
            .expect("the update works");
        assert_eq!(missing, None);
    })
    .await;
}

#[tokio::test]
async fn changed_user_cannot_take_a_name_or_a_group_that_is_not_free() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let users = database.users();
        users
            .insert(new_user("zoe", group.id))
            .await
            .expect("the user can be stored");
        let yan = users
            .insert(new_user("yan", group.id))
            .await
            .expect("the user can be stored");

        let taken = users
            .update(
                yan.id,
                UserChanges {
                    username: Change::Set("ZOE".to_owned()),
                    ..UserChanges::none()
                },
            )
            .await;
        assert!(matches!(taken, Err(DbError::Duplicate)), "{taken:?}");
        let no_group = users
            .update(
                yan.id,
                UserChanges {
                    group: Change::Set(GroupId::new(group.id.get() + 100)),
                    ..UserChanges::none()
                },
            )
            .await;
        assert!(
            matches!(no_group, Err(DbError::ReferenceViolation)),
            "{no_group:?}"
        );
        // A user can take the own name in another letter case.
        let renamed = users
            .update(
                yan.id,
                UserChanges {
                    username: Change::Set("YAN".to_owned()),
                    ..UserChanges::none()
                },
            )
            .await
            .expect("the update works")
            .expect("the user exists");
        assert_eq!(renamed.username, "YAN");
    })
    .await;
}

#[tokio::test]
async fn name_or_address_of_another_user_is_taken_for_both_uses() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let users = database.users();
        let zoe = users
            .insert(NewUser {
                email: Some("Zoe@Example.com".to_owned()),
                ..new_user("zoe", group.id)
            })
            .await
            .expect("the user can be stored");
        let yan = users
            .insert(new_user("yan", group.id))
            .await
            .expect("the user can be stored");
        let taken = async |text: &str, except: UserId| {
            users
                .is_taken_by_another(text, except)
                .await
                .expect("the lookup works")
        };

        // A person logs in with a name or an address, so neither can be both.
        assert!(taken("ZOE", yan.id).await, "the name of another user");
        assert!(
            taken("zoe@example.com", yan.id).await,
            "the address of another user"
        );
        assert!(!taken("zoe", zoe.id).await, "the own name");
        assert!(!taken("ZOE@EXAMPLE.COM", zoe.id).await, "the own address");
        assert!(!taken("nobody", yan.id).await);
        assert!(!taken("", yan.id).await);
    })
    .await;
}

#[tokio::test]
async fn users_are_listed_in_the_order_in_which_they_were_stored() {
    on_each_engine(async |database| {
        let group = insert_group(database, 4, "User").await;
        let users = database.users();
        assert_eq!(users.list().await.expect("the list works"), []);

        for name in ["zoe", "abe", "yan"] {
            users
                .insert(new_user(name, group.id))
                .await
                .expect("the user can be stored");
        }

        let names: Vec<String> = users
            .list()
            .await
            .expect("the list works")
            .into_iter()
            .map(|user| user.username)
            .collect();
        assert_eq!(names, ["zoe", "abe", "yan"]);
    })
    .await;
}
