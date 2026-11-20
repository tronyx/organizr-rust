//! Bookmarks and their categories must be stored the same way on `SQLite` and on
//! `PostgreSQL`.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::DbError;
use organizr_domain::{BookmarkCategoryChanges, BookmarkChanges, NewBookmark, NewBookmarkCategory};
use organizr_testkit::engines::on_each_engine;

fn category(number: i64, name: &str, is_default: bool) -> NewBookmarkCategory {
    NewBookmarkCategory {
        number,
        position: number,
        name: name.to_owned(),
        is_default,
    }
}

fn bookmark(position: i64, name: &str, category: i64) -> NewBookmark {
    NewBookmark {
        position,
        category,
        name: name.to_owned(),
        url: format!("https://{}.example.com", name.to_lowercase()),
        enabled: true,
        least_access: 4,
        image: "fontawesome::bookmark".to_owned(),
        background_color: "#fff".to_owned(),
        text_color: "#000000".to_owned(),
    }
}

#[tokio::test]
async fn bookmark_is_stored_changed_and_deleted() {
    on_each_engine(async |database| {
        let bookmarks = database.bookmarks();
        assert_eq!(bookmarks.next_position().await.expect("it works"), 1);

        let wiki = bookmarks
            .insert(bookmark(1, "Wiki", 1))
            .await
            .expect("it is stored");
        let news = bookmarks
            .insert(bookmark(2, "News", 1))
            .await
            .expect("it is stored");
        assert_eq!(wiki.url, "https://wiki.example.com");
        assert_eq!(wiki.least_access, 4);
        assert_eq!(bookmarks.next_position().await.expect("it works"), 3);
        assert_eq!(
            bookmarks.find(wiki.id).await.expect("it loads"),
            Some(wiki.clone())
        );

        // A name is taken whatever its letter case, and a bookmark keeps its own.
        assert!(
            bookmarks
                .is_name_taken("WIKI", None)
                .await
                .expect("it works")
        );
        assert!(
            !bookmarks
                .is_name_taken("wiki", Some(wiki.id))
                .await
                .expect("it works")
        );
        assert!(matches!(
            bookmarks.insert(bookmark(3, "wiki", 1)).await,
            Err(DbError::Duplicate)
        ));

        let changed = bookmarks
            .update(
                wiki.id,
                BookmarkChanges {
                    name: Some("Docs".to_owned()),
                    enabled: Some(false),
                    least_access: Some(1),
                    background_color: Some("#123456".to_owned()),
                    ..BookmarkChanges::default()
                },
            )
            .await
            .expect("it is changed")
            .expect("the bookmark exists");
        assert_eq!(changed.name, "Docs");
        assert!(!changed.enabled);
        assert_eq!(changed.least_access, 1);
        assert_eq!(changed.background_color, "#123456");
        // What the change did not name stays.
        assert_eq!(changed.url, wiki.url);
        assert!(
            !bookmarks
                .is_name_taken("wiki", None)
                .await
                .expect("it works")
        );
        let nothing = BookmarkChanges::default();
        assert_eq!(
            bookmarks.update(9999, nothing).await.expect("it works"),
            None
        );

        bookmarks
            .set_positions(&[(wiki.id, 2), (news.id, 1), (9999, 7)])
            .await
            .expect("the places are set");
        let names: Vec<String> = bookmarks
            .list()
            .await
            .expect("it loads")
            .into_iter()
            .map(|one| one.name)
            .collect();
        assert_eq!(names, ["News", "Docs"]);

        assert!(bookmarks.delete(wiki.id).await.expect("it works"));
        assert!(!bookmarks.delete(wiki.id).await.expect("it works"));
        assert_eq!(bookmarks.list().await.expect("it loads").len(), 1);
    })
    .await;
}

#[tokio::test]
async fn one_category_is_always_the_default() {
    on_each_engine(async |database| {
        let bookmarks = database.bookmarks();
        assert_eq!(bookmarks.default_category().await.expect("it works"), None);
        assert_eq!(bookmarks.next_category_number().await.expect("it works"), 1);

        // The first category is the default, also when it does not say so.
        let work = bookmarks
            .insert_category(category(1, "Work", false))
            .await
            .expect("it is stored");
        assert!(work.is_default);
        // A new default takes the mark from the other one.
        let home = bookmarks
            .insert_category(category(2, "Home", true))
            .await
            .expect("it is stored");
        assert!(home.is_default);
        assert_eq!(
            bookmarks.default_category().await.expect("it works"),
            Some(2)
        );
        let defaults = |list: Vec<organizr_domain::BookmarkCategory>| {
            list.into_iter()
                .filter(|one| one.is_default)
                .map(|one| one.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            defaults(bookmarks.categories().await.expect("it loads")),
            ["Home"]
        );
        assert_eq!(bookmarks.next_category_number().await.expect("it works"), 3);
        assert_eq!(
            bookmarks.next_category_position().await.expect("it works"),
            3
        );

        assert!(
            bookmarks
                .is_category_name_taken("HOME", None)
                .await
                .expect("it works")
        );
        assert!(
            bookmarks
                .is_category_number_taken(2, None)
                .await
                .expect("it works")
        );
        assert!(
            !bookmarks
                .is_category_number_taken(2, Some(home.id))
                .await
                .expect("it works")
        );
        assert!(matches!(
            bookmarks.insert_category(category(2, "Other", false)).await,
            Err(DbError::Duplicate)
        ));

        // A category that gives the mark away leaves it to the lowest number.
        let changed = bookmarks
            .update_category(
                home.id,
                BookmarkCategoryChanges {
                    name: Some("Private".to_owned()),
                    is_default: Some(false),
                    ..BookmarkCategoryChanges::default()
                },
            )
            .await
            .expect("it is changed")
            .expect("the category exists");
        assert_eq!(changed.name, "Private");
        assert!(!changed.is_default);
        assert_eq!(
            bookmarks.default_category().await.expect("it works"),
            Some(1)
        );

        // The default category goes, and the other one takes the mark. Its bookmark
        // stays.
        let kept = bookmarks
            .insert(bookmark(1, "Wiki", 1))
            .await
            .expect("it is stored");
        assert!(bookmarks.delete_category(work.id).await.expect("it works"));
        assert!(!bookmarks.delete_category(work.id).await.expect("it works"));
        assert_eq!(
            bookmarks.default_category().await.expect("it works"),
            Some(2)
        );
        assert_eq!(bookmarks.find(kept.id).await.expect("it loads"), Some(kept));
        assert_eq!(
            bookmarks
                .find_category(home.id)
                .await
                .expect("it loads")
                .map(|one| one.is_default),
            Some(true)
        );
    })
    .await;
}

#[tokio::test]
async fn categories_take_new_places() {
    on_each_engine(async |database| {
        let bookmarks = database.bookmarks();
        let work = bookmarks
            .insert_category(category(1, "Work", false))
            .await
            .expect("it is stored");
        let home = bookmarks
            .insert_category(category(2, "Home", false))
            .await
            .expect("it is stored");

        bookmarks
            .set_category_positions(&[(work.id, 9), (home.id, 3), (9999, 1)])
            .await
            .expect("the places are set");

        let names: Vec<String> = bookmarks
            .categories()
            .await
            .expect("it loads")
            .into_iter()
            .map(|one| one.name)
            .collect();
        assert_eq!(names, ["Home", "Work"]);
        assert_eq!(
            bookmarks.next_category_position().await.expect("it works"),
            10
        );
    })
    .await;
}
