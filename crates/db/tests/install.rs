//! The set-up of a new install, on both engines.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::{Database, DbError, NewInstall, StoredSetting};
use organizr_domain::{GroupLevel, NewGroup, PasswordHash, TabKind, Timestamp};
use organizr_testkit::engines::on_each_engine;

const NOW: Timestamp = Timestamp::from_unix_seconds(1_800_000_000);

fn new_install() -> NewInstall {
    NewInstall {
        admin_username: "Chris".to_owned(),
        admin_email: "chris@example.com".to_owned(),
        admin_password_hash: PasswordHash::new("$argon2id$v=19$not.a.real.hash".to_owned()),
        admin_image: Some("https://example.com/chris.png".to_owned()),
        created_at: NOW,
        settings: vec![
            StoredSetting {
                key: "uuid".to_owned(),
                value: "\"5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21\"".to_owned(),
            },
            StoredSetting {
                key: "license".to_owned(),
                value: "\"personal\"".to_owned(),
            },
        ],
    }
}

async fn is_done(database: &Database) -> bool {
    database
        .install()
        .is_done()
        .await
        .expect("the question can be answered")
}

#[tokio::test]
async fn set_up_stores_what_a_new_install_starts_with() {
    on_each_engine(async |database| {
        assert!(!is_done(database).await);

        let admin = database
            .install()
            .run(new_install())
            .await
            .expect("the set-up works");

        assert!(is_done(database).await);

        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");
        let summary: Vec<(u16, &str, bool)> = groups
            .iter()
            .map(|group| (group.level.get(), group.name.as_str(), group.is_default))
            .collect();
        assert_eq!(
            summary,
            [
                (0, "Admin", false),
                (1, "Co-Admin", false),
                (2, "Super User", false),
                (3, "Power User", false),
                (4, "User", true),
                (999, "Guest", false),
            ]
        );
        assert_eq!(
            groups[5].image.as_deref(),
            Some("plugins/images/groups/guest.png")
        );

        assert_eq!(admin.username, "Chris");
        assert_eq!(admin.email.as_deref(), Some("chris@example.com"));
        assert_eq!(
            admin.password_hash.expose(),
            "$argon2id$v=19$not.a.real.hash"
        );
        assert_eq!(
            admin.image.as_deref(),
            Some("https://example.com/chris.png")
        );
        assert_eq!(admin.group, groups[0].id, "the first account is an Admin");
        assert_eq!(admin.created_at, NOW);
        assert!(!admin.locked);
        let found = database
            .users()
            .find_by_login("CHRIS@example.com")
            .await
            .expect("the lookup works");
        assert_eq!(found, Some(admin));

        let categories = database
            .categories()
            .list()
            .await
            .expect("the categories can be read");
        assert_eq!(categories.len(), 1);
        assert_eq!(categories[0].name, "Unsorted");
        assert_eq!(categories[0].number.get(), 0);
        assert!(categories[0].is_default);

        let tabs = database.tabs().list().await.expect("the tabs can be read");
        let summary: Vec<(&str, Option<&str>, bool, u16)> = tabs
            .iter()
            .map(|tab| {
                (
                    tab.name.as_str(),
                    tab.url.as_deref(),
                    tab.enabled,
                    tab.access.least_access().get(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("Settings", Some("api/v2/page/settings"), true, 1),
                ("Homepage", Some("api/v2/page/homepage"), false, 4),
            ]
        );
        for tab in &tabs {
            assert_eq!(tab.category, categories[0].id);
            assert_eq!(tab.kind, TabKind::Internal);
        }
    })
    .await;
}

#[tokio::test]
async fn set_up_stores_the_settings_it_is_given() {
    on_each_engine(async |database| {
        database
            .install()
            .run(new_install())
            .await
            .expect("the set-up works");

        let settings = database
            .settings()
            .all()
            .await
            .expect("the settings can be read");
        assert_eq!(
            settings,
            [
                StoredSetting {
                    key: "license".to_owned(),
                    value: "\"personal\"".to_owned(),
                },
                StoredSetting {
                    key: "uuid".to_owned(),
                    value: "\"5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21\"".to_owned(),
                },
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn install_cannot_be_set_up_twice() {
    on_each_engine(async |database| {
        database
            .install()
            .run(new_install())
            .await
            .expect("the first set-up works");

        let second = database
            .install()
            .run(NewInstall {
                admin_username: "intruder".to_owned(),
                admin_email: "intruder@example.com".to_owned(),
                settings: vec![StoredSetting {
                    key: "uuid".to_owned(),
                    value: "\"another\"".to_owned(),
                }],
                ..new_install()
            })
            .await;

        assert!(matches!(second, Err(DbError::AlreadySetUp)), "{second:?}");
        assert_eq!(database.users().count().await.expect("the count works"), 1);
        let intruder = database
            .users()
            .find_by_username("intruder")
            .await
            .expect("the lookup works");
        assert_eq!(intruder, None);
        let settings = database
            .settings()
            .all()
            .await
            .expect("the settings can be read");
        assert!(
            settings
                .iter()
                .all(|setting| setting.value != "\"another\""),
            "{settings:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn set_up_that_fails_stores_nothing() {
    on_each_engine(async |database| {
        // A group with the name of a group of a new install is in the way. The set-up
        // stores four groups before it gets there.
        let in_the_way = database
            .groups()
            .insert(NewGroup {
                level: GroupLevel::new(500).expect("the level is valid"),
                name: "user".to_owned(),
                image: None,
                is_default: false,
            })
            .await
            .expect("the group can be stored");

        let result = database.install().run(new_install()).await;

        assert!(matches!(result, Err(DbError::AlreadySetUp)), "{result:?}");
        assert!(!is_done(database).await);
        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");
        assert_eq!(groups, [in_the_way], "no group of the set-up is left");
        assert_eq!(database.users().count().await.expect("the count works"), 0);
        let settings = database
            .settings()
            .all()
            .await
            .expect("the settings can be read");
        assert_eq!(settings, []);

        // Once the group is gone, the set-up works.
        let deleted = database
            .groups()
            .delete(groups[0].id)
            .await
            .expect("the delete works");
        assert!(deleted);
        database
            .install()
            .run(new_install())
            .await
            .expect("the set-up works now");
        assert!(is_done(database).await);
    })
    .await;
}
