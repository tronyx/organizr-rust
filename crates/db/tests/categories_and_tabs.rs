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
    Category, CategoryChanges, CategoryId, CategoryNumber, Change, GroupLevel, LevelRange,
    NewCategory, NewTab, Tab, TabChanges, TabId, TabKind,
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

async fn insert_tab(database: &Database, position: i64, name: &str, category: CategoryId) -> Tab {
    database
        .tabs()
        .insert(new_tab(position, name, category))
        .await
        .expect("the tab can be stored")
}

/// The names of the tabs that are marked as the default, in menu order.
async fn default_tabs(database: &Database) -> Vec<String> {
    database
        .tabs()
        .list()
        .await
        .expect("the tabs can be read")
        .into_iter()
        .filter(|tab| tab.is_default)
        .map(|tab| tab.name)
        .collect()
}

#[tokio::test]
async fn category_is_found_by_its_number() {
    on_each_engine(async |database| {
        let media = insert_category(database, 7, 1, "Media").await;
        insert_category(database, 0, 2, "Unsorted").await;

        let found = database
            .categories()
            .find_by_number(CategoryNumber::new(7))
            .await
            .expect("the lookup works");
        let missing = database
            .categories()
            .find_by_number(CategoryNumber::new(media.id.get()))
            .await
            .expect("the lookup works");

        assert_eq!(found, Some(media));
        assert_eq!(missing, None, "the id of a category is not its number");
    })
    .await;
}

#[tokio::test]
async fn default_category_is_the_one_that_is_marked() {
    on_each_engine(async |database| {
        let default = async || {
            database
                .categories()
                .find_default()
                .await
                .expect("the lookup works")
                .map(|category| category.name)
        };
        assert_eq!(default().await, None, "no category yet");
        insert_category(database, 0, 1, "Unsorted").await;
        assert_eq!(default().await, None, "no category is marked");

        // The mark moves to the category that was stored last as the default.
        for (number, position, name) in [(1, 3, "Media"), (2, 2, "Books")] {
            database
                .categories()
                .insert(NewCategory {
                    is_default: true,
                    ..new_category(number, position, name)
                })
                .await
                .expect("the category can be stored");
            assert_eq!(default().await.as_deref(), Some(name));
        }
    })
    .await;
}

#[tokio::test]
async fn changed_fields_of_a_tab_are_stored_and_the_others_stay() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let media = insert_category(database, 1, 2, "Media").await;
        let plex = insert_tab(database, 3, "Plex", unsorted.id).await;
        let access = LevelRange::new(level(999), level(2)).expect("the range is valid");

        let changed = database
            .tabs()
            .update(
                plex.id,
                TabChanges {
                    position: Change::Set(9),
                    category: Change::Set(media.id),
                    name: Change::Set("Plex Media".to_owned()),
                    url: Change::Set(Some("https://plex.example.com/".to_owned())),
                    enabled: Change::Set(true),
                    access: Change::Set(access),
                    kind: Change::Set(TabKind::NewWindow),
                    timeout_ms: Change::Set(Some(60_000)),
                    ..TabChanges::default()
                },
            )
            .await
            .expect("the tab can be changed")
            .expect("the tab exists");

        let expected = Tab {
            position: 9,
            category: media.id,
            name: "Plex Media".to_owned(),
            url: Some("https://plex.example.com/".to_owned()),
            enabled: true,
            access,
            kind: TabKind::NewWindow,
            timeout_ms: Some(60_000),
            ..plex.clone()
        };
        assert_eq!(changed, expected);
        let stored = database
            .tabs()
            .find(plex.id)
            .await
            .expect("the lookup works");
        assert_eq!(stored, Some(expected));
    })
    .await;
}

#[tokio::test]
async fn field_of_a_tab_can_be_made_empty() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let full = database
            .tabs()
            .insert(NewTab {
                url: Some("https://full.example.com/".to_owned()),
                url_local: Some("http://10.0.0.6/".to_owned()),
                image: Some("fontawesome::film".to_owned()),
                ping_url: Some("10.0.0.6:80".to_owned()),
                timeout_ms: Some(60_000),
                ..new_tab(1, "Full", unsorted.id)
            })
            .await
            .expect("the tab can be stored");

        let changed = database
            .tabs()
            .update(
                full.id,
                TabChanges {
                    url_local: Change::Set(None),
                    ping_url: Change::Set(None),
                    timeout_ms: Change::Set(None),
                    ..TabChanges::default()
                },
            )
            .await
            .expect("the tab can be changed")
            .expect("the tab exists");

        assert_eq!(
            changed,
            Tab {
                url_local: None,
                ping_url: None,
                timeout_ms: None,
                ..full
            }
        );
    })
    .await;
}

#[tokio::test]
async fn change_without_fields_leaves_the_tab_as_it_is() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;

        let changed = database
            .tabs()
            .update(plex.id, TabChanges::default())
            .await
            .expect("an empty change works");

        assert_eq!(changed, Some(plex));
    })
    .await;
}

#[tokio::test]
async fn change_of_a_tab_that_does_not_exist_changes_nothing() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;

        let changed = database
            .tabs()
            .update(
                TabId::new(plex.id.get() + 1),
                TabChanges {
                    name: Change::Set("Nobody".to_owned()),
                    is_default: Change::Set(true),
                    ..TabChanges::default()
                },
            )
            .await
            .expect("the change works");

        assert_eq!(changed, None);
        assert_eq!(
            database.tabs().list().await.expect("the tabs can be read"),
            [plex]
        );
    })
    .await;
}

#[tokio::test]
async fn tab_cannot_take_the_name_of_another_tab_whatever_the_letter_case() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;
        let sonarr = insert_tab(database, 2, "Sonarr", unsorted.id).await;
        let rename = |name: &str| TabChanges {
            name: Change::Set(name.to_owned()),
            position: Change::Set(5),
            ..TabChanges::default()
        };

        let taken = database.tabs().update(sonarr.id, rename("PLEX")).await;
        assert!(matches!(taken, Err(DbError::Duplicate)), "{taken:?}");
        // Nothing of the refused change is stored.
        assert_eq!(
            database
                .tabs()
                .find(sonarr.id)
                .await
                .expect("the lookup works"),
            Some(sonarr)
        );

        // A tab can change the letter case of its own name.
        let own = database
            .tabs()
            .update(plex.id, rename("PLEX"))
            .await
            .expect("the tab can be changed")
            .expect("the tab exists");
        assert_eq!(own.name, "PLEX");
    })
    .await;
}

#[tokio::test]
async fn tab_cannot_move_to_a_category_that_does_not_exist() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;

        let result = database
            .tabs()
            .update(
                plex.id,
                TabChanges {
                    category: Change::Set(CategoryId::new(unsorted.id.get() + 1)),
                    ..TabChanges::default()
                },
            )
            .await;

        assert!(
            matches!(result, Err(DbError::ReferenceViolation)),
            "{result:?}"
        );
    })
    .await;
}

#[tokio::test]
async fn only_one_tab_is_the_default() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;
        let default = |is_default| TabChanges {
            is_default: Change::Set(is_default),
            ..TabChanges::default()
        };
        assert_eq!(default_tabs(database).await, Vec::<String>::new());

        // A new tab that is the default.
        let sonarr = database
            .tabs()
            .insert(NewTab {
                is_default: true,
                ..new_tab(2, "Sonarr", unsorted.id)
            })
            .await
            .expect("the tab can be stored");
        assert_eq!(default_tabs(database).await, ["Sonarr"]);

        // Another tab becomes the default.
        let changed = database.tabs().update(plex.id, default(true)).await;
        assert!(matches!(changed, Ok(Some(_))), "{changed:?}");
        assert_eq!(default_tabs(database).await, ["Plex"]);

        // A second new tab that is the default.
        database
            .tabs()
            .insert(NewTab {
                is_default: true,
                ..new_tab(3, "Radarr", unsorted.id)
            })
            .await
            .expect("the tab can be stored");
        assert_eq!(default_tabs(database).await, ["Radarr"]);

        // Taking the mark from a tab that does not have it leaves the others alone.
        let changed = database.tabs().update(sonarr.id, default(false)).await;
        assert!(matches!(changed, Ok(Some(_))), "{changed:?}");
        assert_eq!(default_tabs(database).await, ["Radarr"]);
    })
    .await;
}

#[tokio::test]
async fn new_default_tab_with_a_taken_name_leaves_the_default_as_it_is() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        database
            .tabs()
            .insert(NewTab {
                is_default: true,
                ..new_tab(1, "Plex", unsorted.id)
            })
            .await
            .expect("the tab can be stored");
        let sonarr = insert_tab(database, 2, "Sonarr", unsorted.id).await;

        // Neither a new tab nor a change that fails takes the mark from "Plex".
        let added = database
            .tabs()
            .insert(NewTab {
                is_default: true,
                ..new_tab(3, "plex", unsorted.id)
            })
            .await;
        assert!(matches!(added, Err(DbError::Duplicate)), "{added:?}");
        let changed = database
            .tabs()
            .update(
                sonarr.id,
                TabChanges {
                    name: Change::Set("PLEX".to_owned()),
                    is_default: Change::Set(true),
                    ..TabChanges::default()
                },
            )
            .await;
        assert!(matches!(changed, Err(DbError::Duplicate)), "{changed:?}");

        assert_eq!(default_tabs(database).await, ["Plex"]);
    })
    .await;
}

#[tokio::test]
async fn name_of_a_tab_is_taken_whatever_the_letter_case() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;
        let sonarr = insert_tab(database, 2, "Sonarr", unsorted.id).await;
        let tabs = database.tabs();
        let taken = async |name: &str, except: Option<TabId>| {
            tabs.is_name_taken(name, except)
                .await
                .expect("the lookup works")
        };

        assert!(taken("Plex", None).await);
        assert!(taken("pLEX", None).await);
        assert!(!taken("Radarr", None).await);
        // A tab can keep its own name, but not take the name of another tab.
        assert!(!taken("PLEX", Some(plex.id)).await);
        assert!(taken("PLEX", Some(sonarr.id)).await);
        // The characters that stand for "anything" in an SQL pattern are plain characters.
        assert!(!taken("Pl_x", None).await);
        assert!(!taken("P%", None).await);
        assert!(!taken("", None).await);
    })
    .await;
}

#[tokio::test]
async fn next_place_comes_after_the_highest_place_in_use() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let next = async || {
            database
                .tabs()
                .next_position()
                .await
                .expect("the lookup works")
        };

        assert_eq!(next().await, 1, "no tab yet");
        insert_tab(database, 7, "Plex", unsorted.id).await;
        insert_tab(database, 3, "Sonarr", unsorted.id).await;
        assert_eq!(next().await, 8);
    })
    .await;
}

#[tokio::test]
async fn tabs_get_new_places_all_at_once() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let plex = insert_tab(database, 1, "Plex", unsorted.id).await;
        let sonarr = insert_tab(database, 2, "Sonarr", unsorted.id).await;
        let radarr = insert_tab(database, 3, "Radarr", unsorted.id).await;
        let unknown = TabId::new(radarr.id.get() + 1);

        database
            .tabs()
            .set_positions(&[(plex.id, 30), (unknown, 5), (sonarr.id, 10)])
            .await
            .expect("the places can be set");

        let order: Vec<(String, i64)> = database
            .tabs()
            .list()
            .await
            .expect("the tabs can be read")
            .into_iter()
            .map(|tab| (tab.name, tab.position))
            .collect();
        assert_eq!(
            order,
            [
                ("Radarr".to_owned(), 3),
                ("Sonarr".to_owned(), 10),
                ("Plex".to_owned(), 30)
            ]
        );
        // An empty list is no error.
        database
            .tabs()
            .set_positions(&[])
            .await
            .expect("an empty list works");
    })
    .await;
}

/// The names of the categories that are marked as the default, in menu order.
async fn default_categories(database: &Database) -> Vec<String> {
    database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
        .into_iter()
        .filter(|category| category.is_default)
        .map(|category| category.name)
        .collect()
}

#[tokio::test]
async fn category_is_found_by_its_id() {
    on_each_engine(async |database| {
        let media = insert_category(database, 7, 1, "Media").await;
        let categories = database.categories();

        let found = categories.find(media.id).await.expect("the lookup works");
        let missing = categories
            .find(CategoryId::new(media.id.get() + 1))
            .await
            .expect("the lookup works");

        assert_eq!(found, Some(media));
        assert_eq!(missing, None);
    })
    .await;
}

#[tokio::test]
async fn changed_fields_of_a_category_are_stored_and_the_others_stay() {
    on_each_engine(async |database| {
        let media = database
            .categories()
            .insert(NewCategory {
                image: Some("fontawesome::film".to_owned()),
                ..new_category(1, 2, "Media")
            })
            .await
            .expect("the category can be stored");

        let changed = database
            .categories()
            .update(
                media.id,
                CategoryChanges {
                    number: Change::Set(CategoryNumber::new(7)),
                    name: Change::Set("Films".to_owned()),
                    ..CategoryChanges::default()
                },
            )
            .await
            .expect("the category can be changed")
            .expect("the category exists");
        let expected = Category {
            number: CategoryNumber::new(7),
            name: "Films".to_owned(),
            ..media.clone()
        };
        assert_eq!(changed, expected);

        let changed = database
            .categories()
            .update(
                media.id,
                CategoryChanges {
                    position: Change::Set(40),
                    image: Change::Set(None),
                    ..CategoryChanges::default()
                },
            )
            .await
            .expect("the category can be changed");
        let expected = Category {
            position: 40,
            image: None,
            ..expected
        };
        assert_eq!(changed, Some(expected.clone()));
        let stored = database
            .categories()
            .find(media.id)
            .await
            .expect("the lookup works");
        assert_eq!(stored, Some(expected));
    })
    .await;
}

#[tokio::test]
async fn change_of_a_category_without_fields_or_without_a_category_changes_nothing() {
    on_each_engine(async |database| {
        let media = insert_category(database, 1, 2, "Media").await;
        let categories = database.categories();

        let same = categories
            .update(media.id, CategoryChanges::default())
            .await
            .expect("an empty change works");
        let unknown = categories
            .update(
                CategoryId::new(media.id.get() + 1),
                CategoryChanges {
                    name: Change::Set("Nobody".to_owned()),
                    is_default: Change::Set(true),
                    ..CategoryChanges::default()
                },
            )
            .await
            .expect("the change works");

        assert_eq!(same, Some(media.clone()));
        assert_eq!(unknown, None);
        assert_eq!(
            categories.list().await.expect("the categories can be read"),
            [media]
        );
    })
    .await;
}

#[tokio::test]
async fn category_cannot_take_the_name_or_the_number_of_another_category() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let media = insert_category(database, 1, 2, "Media").await;
        let categories = database.categories();

        let name = categories
            .update(
                media.id,
                CategoryChanges {
                    name: Change::Set("UNSORTED".to_owned()),
                    position: Change::Set(9),
                    ..CategoryChanges::default()
                },
            )
            .await;
        let number = categories
            .update(
                media.id,
                CategoryChanges {
                    number: Change::Set(unsorted.number),
                    position: Change::Set(9),
                    ..CategoryChanges::default()
                },
            )
            .await;
        assert!(matches!(name, Err(DbError::Duplicate)), "{name:?}");
        assert!(matches!(number, Err(DbError::Duplicate)), "{number:?}");
        // Nothing of the refused changes is stored.
        assert_eq!(
            categories.find(media.id).await.expect("the lookup works"),
            Some(media.clone())
        );

        // A category can change the letter case of its own name and keep its number.
        let own = categories
            .update(
                media.id,
                CategoryChanges {
                    name: Change::Set("MEDIA".to_owned()),
                    number: Change::Set(media.number),
                    ..CategoryChanges::default()
                },
            )
            .await
            .expect("the category can be changed")
            .expect("the category exists");
        assert_eq!(own.name, "MEDIA");
    })
    .await;
}

#[tokio::test]
async fn only_one_category_is_the_default() {
    on_each_engine(async |database| {
        let unsorted = database
            .categories()
            .insert(NewCategory {
                is_default: true,
                ..new_category(0, 1, "Unsorted")
            })
            .await
            .expect("the category can be stored");
        let default = |is_default| CategoryChanges {
            is_default: Change::Set(is_default),
            ..CategoryChanges::default()
        };
        assert_eq!(default_categories(database).await, ["Unsorted"]);

        // A new category that is the default.
        let media = database
            .categories()
            .insert(NewCategory {
                is_default: true,
                ..new_category(1, 2, "Media")
            })
            .await
            .expect("the category can be stored");
        assert_eq!(default_categories(database).await, ["Media"]);

        // Another category becomes the default.
        let changed = database
            .categories()
            .update(unsorted.id, default(true))
            .await;
        assert!(matches!(changed, Ok(Some(_))), "{changed:?}");
        assert_eq!(default_categories(database).await, ["Unsorted"]);

        // Taking the mark from a category that does not have it leaves the others alone.
        let changed = database.categories().update(media.id, default(false)).await;
        assert!(matches!(changed, Ok(Some(_))), "{changed:?}");
        assert_eq!(default_categories(database).await, ["Unsorted"]);

        // A new default category with a taken name does not take the mark.
        let added = database
            .categories()
            .insert(NewCategory {
                is_default: true,
                ..new_category(2, 3, "MEDIA")
            })
            .await;
        assert!(matches!(added, Err(DbError::Duplicate)), "{added:?}");
        assert_eq!(default_categories(database).await, ["Unsorted"]);
    })
    .await;
}

#[tokio::test]
async fn name_and_number_of_a_category_are_taken_once() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let media = insert_category(database, 7, 2, "Media").await;
        let categories = database.categories();
        let name_taken = async |name: &str, except: Option<CategoryId>| {
            categories
                .is_name_taken(name, except)
                .await
                .expect("the lookup works")
        };
        let number_taken = async |number: i64, except: Option<CategoryId>| {
            categories
                .is_number_taken(CategoryNumber::new(number), except)
                .await
                .expect("the lookup works")
        };

        assert!(name_taken("mEDIA", None).await);
        assert!(!name_taken("Books", None).await);
        assert!(!name_taken("MEDIA", Some(media.id)).await);
        assert!(name_taken("MEDIA", Some(unsorted.id)).await);
        // The characters that stand for "anything" in an SQL pattern are plain characters.
        assert!(!name_taken("M_dia", None).await);
        assert!(!name_taken("%", None).await);

        assert!(number_taken(7, None).await);
        assert!(number_taken(0, None).await);
        assert!(!number_taken(1, None).await);
        assert!(!number_taken(7, Some(media.id)).await);
        assert!(number_taken(7, Some(unsorted.id)).await);
    })
    .await;
}

#[tokio::test]
async fn next_place_and_next_number_of_a_category_come_after_the_highest() {
    on_each_engine(async |database| {
        let categories = database.categories();
        let next = async || {
            (
                categories.next_position().await.expect("the lookup works"),
                categories
                    .next_number()
                    .await
                    .expect("the lookup works")
                    .get(),
            )
        };

        assert_eq!(next().await, (1, 1), "no category yet");
        insert_category(database, 0, 1, "Unsorted").await;
        assert_eq!(next().await, (2, 1));
        insert_category(database, 20, 9, "Media").await;
        insert_category(database, 3, 4, "Books").await;
        assert_eq!(next().await, (10, 21));
    })
    .await;
}

#[tokio::test]
async fn categories_get_new_places_all_at_once() {
    on_each_engine(async |database| {
        let unsorted = insert_category(database, 0, 1, "Unsorted").await;
        let media = insert_category(database, 1, 2, "Media").await;
        let books = insert_category(database, 2, 3, "Books").await;
        let unknown = CategoryId::new(books.id.get() + 1);

        database
            .categories()
            .set_positions(&[(unsorted.id, 30), (unknown, 5), (media.id, 10)])
            .await
            .expect("the places can be set");

        let order: Vec<(String, i64)> = database
            .categories()
            .list()
            .await
            .expect("the categories can be read")
            .into_iter()
            .map(|category| (category.name, category.position))
            .collect();
        assert_eq!(
            order,
            [
                ("Books".to_owned(), 3),
                ("Media".to_owned(), 10),
                ("Unsorted".to_owned(), 30)
            ]
        );
    })
    .await;
}
