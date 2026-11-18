//! Categories and tabs must be stored the same way on `SQLite` and on `PostgreSQL`.
//!
//! Every test here runs once per engine. See `organizr_testkit::engines` for how the
//! engines are chosen.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::{Database, DbError};
use organizr_domain::{
    Category, CategoryId, CategoryNumber, GroupLevel, LevelRange, NewCategory, NewTab, TabId,
    TabKind,
};
use organizr_testkit::engines::on_each_engine;

fn level(value: u16) -> GroupLevel {
    GroupLevel::new(value).expect("the level is valid")
}

/// A category with the given number, position and name, and nothing else set.
fn new_category(number: i64, position: i64, name: &str) -> NewCategory {
    NewCategory {
        number: CategoryNumber::new(number),
        position,
        name: name.to_owned(),
        image: None,
        is_default: false,
    }
}

/// A tab with the given position and name in the given category, and nothing else set.
fn new_tab(position: i64, name: &str, category: CategoryId) -> NewTab {
    NewTab {
        position,
        category,
        name: name.to_owned(),
        url: None,
        url_local: None,
        is_default: false,
        enabled: false,
        access: LevelRange::up_to(level(4)),
        add_to_admin: false,
        image: None,
        kind: TabKind::Iframe,
        splash: false,
        ping: false,
        ping_url: None,
        timeout: false,
        timeout_ms: None,
        preload: false,
    }
}

async fn insert_category(database: &Database, number: i64, position: i64, name: &str) -> Category {
    database
        .categories()
        .insert(new_category(number, position, name))
        .await
        .expect("the category can be stored")
}

#[tokio::test]
async fn category_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let stored = database
            .categories()
            .insert(NewCategory {
                number: CategoryNumber::new(0),
                position: 1,
                name: "Unsorted".to_owned(),
                image: Some("fontawesome::question".to_owned()),
                is_default: true,
            })
            .await
            .expect("the category can be stored");

        assert_eq!(stored.number, CategoryNumber::new(0));
        assert_eq!(stored.position, 1);
        assert_eq!(stored.name, "Unsorted");
        assert_eq!(stored.image.as_deref(), Some("fontawesome::question"));
        assert!(stored.is_default);

        let listed = database
            .categories()
            .list()
            .await
            .expect("the categories can be read");
        assert_eq!(listed, [stored]);
    })
    .await;
}

#[tokio::test]
async fn categories_are_listed_in_menu_order() {
    on_each_engine(async |database| {
        insert_category(database, 0, 3, "Last").await;
        insert_category(database, 1, 1, "First").await;
        insert_category(database, 2, 2, "Middle").await;

        let categories = database
            .categories()
            .list()
            .await
            .expect("the categories can be read");

        let names: Vec<&str> = categories
            .iter()
            .map(|category| category.name.as_str())
            .collect();
        assert_eq!(names, ["First", "Middle", "Last"]);
    })
    .await;
}

#[tokio::test]
async fn two_categories_cannot_share_a_number() {
    on_each_engine(async |database| {
        insert_category(database, 5, 1, "Media").await;

        let result = database
            .categories()
            .insert(new_category(5, 2, "Downloads"))
            .await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn two_categories_cannot_share_a_name_whatever_the_letter_case() {
    on_each_engine(async |database| {
        insert_category(database, 1, 1, "Media").await;

        let result = database
            .categories()
            .insert(new_category(2, 2, "MEDIA"))
            .await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn tab_with_every_field_set_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let category = insert_category(database, 1, 1, "Media").await;
        let new = NewTab {
            position: 7,
            category: category.id,
            name: "Plex Média".to_owned(),
            url: Some("https://plex.example.com/web/index.html#!/".to_owned()),
            url_local: Some("http://10.0.0.5:32400/web".to_owned()),
            is_default: true,
            enabled: true,
            access: LevelRange::new(level(4), level(1)).expect("the range is valid"),
            add_to_admin: true,
            image: Some("plugins/images/tabs/plex.png".to_owned()),
            kind: TabKind::NewWindow,
            splash: true,
            ping: true,
            ping_url: Some("10.0.0.5:32400".to_owned()),
            timeout: true,
            // More than a 32-bit column holds.
            timeout_ms: Some(3_000_000_000),
            preload: true,
        };

        let stored = database
            .tabs()
            .insert(new.clone())
            .await
            .expect("the tab can be stored");

        assert_eq!(stored.position, new.position);
        assert_eq!(stored.category, category.id);
        assert_eq!(stored.name, new.name);
        assert_eq!(stored.url, new.url);
        assert_eq!(stored.url_local, new.url_local);
        assert_eq!(stored.access, new.access);
        assert_eq!(stored.image, new.image);
        assert_eq!(stored.kind, TabKind::NewWindow);
        assert_eq!(stored.ping_url, new.ping_url);
        assert_eq!(stored.timeout_ms, Some(3_000_000_000));
        assert!(stored.is_default && stored.enabled && stored.add_to_admin);
        assert!(stored.splash && stored.ping && stored.timeout && stored.preload);

        let found = database
            .tabs()
            .find(stored.id)
            .await
            .expect("the tab can be read");
        assert_eq!(found, Some(stored));
    })
    .await;
}

#[tokio::test]
async fn tab_with_only_the_required_fields_comes_back_as_it_was_stored() {
    on_each_engine(async |database| {
        let category = insert_category(database, 0, 1, "Unsorted").await;

        let stored = database
            .tabs()
            .insert(NewTab {
                kind: TabKind::Internal,
                ..new_tab(1, "Settings", category.id)
            })
            .await
            .expect("the tab can be stored");

        assert_eq!(stored.url, None);
        assert_eq!(stored.url_local, None);
        assert_eq!(stored.image, None);
        assert_eq!(stored.ping_url, None);
        assert_eq!(stored.timeout_ms, None);
        assert_eq!(stored.kind, TabKind::Internal);
        assert_eq!(stored.access, LevelRange::up_to(level(4)));
        assert!(!stored.is_default && !stored.enabled && !stored.add_to_admin);
        assert!(!stored.splash && !stored.ping && !stored.timeout && !stored.preload);
    })
    .await;
}

#[tokio::test]
async fn unknown_id_finds_no_tab() {
    on_each_engine(async |database| {
        let found = database
            .tabs()
            .find(TabId::new(12_345))
            .await
            .expect("the lookup works");

        assert_eq!(found, None);
    })
    .await;
}

#[tokio::test]
async fn tabs_are_listed_in_menu_order() {
    on_each_engine(async |database| {
        let category = insert_category(database, 0, 1, "Unsorted").await;
        for (position, name) in [(3, "Last"), (1, "First"), (2, "Middle")] {
            database
                .tabs()
                .insert(new_tab(position, name, category.id))
                .await
                .expect("the tab can be stored");
        }

        let tabs = database.tabs().list().await.expect("the tabs can be read");

        let names: Vec<&str> = tabs.iter().map(|tab| tab.name.as_str()).collect();
        assert_eq!(names, ["First", "Middle", "Last"]);
    })
    .await;
}

#[tokio::test]
async fn two_tabs_cannot_share_a_name_whatever_the_letter_case() {
    on_each_engine(async |database| {
        let category = insert_category(database, 0, 1, "Unsorted").await;
        database
            .tabs()
            .insert(new_tab(1, "Sonarr", category.id))
            .await
            .expect("the first tab can be stored");

        let result = database
            .tabs()
            .insert(new_tab(2, "SONARR", category.id))
            .await;

        assert!(matches!(result, Err(DbError::Duplicate)), "{result:?}");
    })
    .await;
}

#[tokio::test]
async fn tab_cannot_join_a_category_that_does_not_exist() {
    on_each_engine(async |database| {
        let result = database
            .tabs()
            .insert(new_tab(1, "Sonarr", CategoryId::new(12_345)))
            .await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn category_with_tabs_cannot_be_deleted() {
    on_each_engine(async |database| {
        let category = insert_category(database, 1, 1, "Media").await;
        database
            .tabs()
            .insert(new_tab(1, "Plex", category.id))
            .await
            .expect("the tab can be stored");

        let result = database.categories().delete(category.id).await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn tab_and_empty_category_can_each_be_deleted_once() {
    on_each_engine(async |database| {
        let category = insert_category(database, 1, 1, "Media").await;
        let tab = database
            .tabs()
            .insert(new_tab(1, "Plex", category.id))
            .await
            .expect("the tab can be stored");

        assert!(
            database
                .tabs()
                .delete(tab.id)
                .await
                .expect("the delete works")
        );
        assert!(
            !database
                .tabs()
                .delete(tab.id)
                .await
                .expect("the delete works")
        );
        assert!(
            database
                .categories()
                .delete(category.id)
                .await
                .expect("the delete works")
        );
        assert!(
            !database
                .categories()
                .delete(category.id)
                .await
                .expect("the delete works")
        );
    })
    .await;
}
