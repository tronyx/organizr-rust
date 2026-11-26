//! The `/categories` routes through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! The recording covers what both applications answer alike. These tests cover callers
//! of other levels, and the requests where this application is stricter than PHP.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{Category, CategoryNumber, GroupLevel, NewCategory, NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{installed_app, seed};
use serde_json::{Value, json};

const PASSWORD: &str = "the password of the test users";

/// Who sends a request.
#[derive(Clone, Copy)]
enum As<'a> {
    /// Nobody: no cookie and no key.
    Guest,
    /// A script with the API key, which counts as the Admin.
    Admin,
    /// A logged-in user, by the session cookie.
    User(&'a str),
}

/// Sends a request to the API. A body is sent as JSON.
async fn call(app: &Router, who: As<'_>, method: &str, path: &str, body: Option<Value>) -> Answer {
    let mut headers: Vec<(&str, &str)> = Vec::new();
    match who {
        As::Guest => {}
        As::Admin => headers.push(("token", seed::API_KEY)),
        As::User(cookie) => headers.push(("cookie", cookie)),
    }
    let text = body.as_ref().map_or_default(Value::to_string);
    if body.is_some() {
        headers.push(("content-type", "application/json"));
    }
    let path = format!("/api/v2{path}");
    send(app, request(method, &path, &headers, &text)).await
}

/// Stores a user in the group of the given level, logs the user in, and returns the
/// session cookie as a browser sends it back.
async fn logged_in(database: &Database, app: &Router, name: &str, group_level: u16) -> String {
    let group = database
        .groups()
        .find_by_level(GroupLevel::new(group_level).expect("the level is valid"))
        .await
        .expect("the lookup works")
        .expect("the seed has the group");
    database
        .users()
        .insert(NewUser {
            username: name.to_owned(),
            email: None,
            password_hash: password::hash(PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
    let body = json!({"username": name, "password": PASSWORD}).to_string();
    let headers = [("content-type", "application/json")];
    let answer = send(app, request("POST", "/api/v2/login", &headers, &body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_") && !cookie.contains("=;"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

/// Stores a category that is not the default.
async fn store_category(database: &Database, number: i64, position: i64, name: &str) -> Category {
    database
        .categories()
        .insert(NewCategory {
            number: CategoryNumber::new(number),
            position,
            name: name.to_owned(),
            image: Some("fontawesome::film".to_owned()),
            is_default: false,
        })
        .await
        .expect("the category can be stored")
}

/// All categories as the database holds them, in menu order.
async fn stored(database: &Database) -> Vec<Category> {
    database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str, context: &str) {
    assert_eq!(answer.status, status, "{context}: {}", answer.body);
    assert_eq!(answer.message(), json!(message), "{context}");
    assert_eq!(
        answer.json()["response"]["result"],
        json!("error"),
        "{context}"
    );
}

#[tokio::test]
async fn category_routes_are_for_admins_and_co_admins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 1, 2, "Media").await;
        let zoe = logged_in(database, &app, "zoe", 4).await;
        let super_user = logged_in(database, &app, "sam", 2).await;
        let co_admin = logged_in(database, &app, "cora", 1).await;
        let before = stored(database).await;
        let one = format!("/categories/{}", media.id);
        let requests = [
            ("GET", "/categories", None),
            (
                "POST",
                "/categories",
                Some(json!({"category": "Books", "image": "i"})),
            ),
            (
                "PUT",
                "/categories",
                Some(json!([{"id": media.id.get(), "order": 50}])),
            ),
            ("PUT", one.as_str(), Some(json!({"category": "Changed"}))),
            ("DELETE", one.as_str(), None),
        ];

        for (method, path, body) in &requests {
            for who in [As::Guest, As::User(&zoe), As::User(&super_user)] {
                let answer = call(&app, who, method, path, body.clone()).await;
                assert_refused(
                    &answer,
                    StatusCode::UNAUTHORIZED,
                    "Not Authorized",
                    &format!("{method} {path}"),
                );
            }
        }
        assert_eq!(stored(database).await, before, "nothing was changed");

        // A Co-Admin passes on every route. The delete comes last.
        for (method, path, body) in requests {
            let answer = call(&app, As::User(&co_admin), method, path, body).await;
            assert!(
                answer.status.is_success(),
                "{method} {path}: {}",
                answer.body
            );
        }
    })
    .await;
}

#[tokio::test]
async fn list_of_categories_is_the_list_of_the_tab_editor() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        store_category(database, 1, 2, "Media").await;

        let categories = call(&app, As::Admin, "GET", "/categories", None).await;
        let tabs = call(&app, As::Admin, "GET", "/tabs", None).await;

        assert_eq!(categories.status, StatusCode::OK);
        assert_eq!(categories.body, tabs.body);
        assert_eq!(
            categories.data()["categories"][1],
            json!({
                "id": categories.data()["categories"][1]["id"],
                "order": 2,
                "category": "Media",
                "category_id": 1,
                "image": "fontawesome::film",
                "default": 0,
            })
        );
    })
    .await;
}

#[tokio::test]
async fn value_of_the_wrong_kind_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 1, 2, "Media").await;
        let path = format!("/categories/{}", media.id);
        let before = stored(database).await;
        let wrong = [
            ("default", json!("yes")),
            ("default", json!(2)),
            ("order", json!("first")),
            ("order", json!(1.5)),
            ("category_id", json!("seven")),
            ("category_id", json!([7])),
            ("category", json!(["Books"])),
            ("image", json!({})),
        ];

        for (field, value) in wrong {
            let mut new = json!({"category": "Books", "image": "i"});
            new[field] = value.clone();
            let added = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
            // A valid field next to it must not be stored either.
            let mut body = if field == "order" {
                json!({"default": 1})
            } else {
                json!({"order": 77})
            };
            body[field] = value.clone();
            let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
            for answer in [added, changed] {
                assert_refused(
                    &answer,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    &format!("Category {field} is not valid"),
                    &format!("{field}: {value}"),
                );
            }
        }
        // A change cannot make these fields empty.
        for field in ["order", "category_id"] {
            let body = json!({field: null});
            let answer = call(&app, As::Admin, "PUT", &path, Some(body)).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("Category {field} is not valid"),
                field,
            );
        }
        assert_eq!(stored(database).await, before, "nothing was stored");
    })
    .await;
}

#[tokio::test]
async fn category_cannot_be_without_a_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 1, 2, "Media").await;
        let path = format!("/categories/{}", media.id);
        let before = stored(database).await;

        // PHP stores a category without a name for each of these.
        for name in [json!(""), json!("  \t "), Value::Null, json!(false)] {
            let new = json!({"category": name, "image": "i"});
            let added = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
            let body = json!({"category": name});
            let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
            for answer in [added, changed] {
                assert_refused(
                    &answer,
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Category name was not supplied",
                    &name.to_string(),
                );
            }
        }
        assert_eq!(stored(database).await, before);
    })
    .await;
}

#[tokio::test]
async fn id_of_a_category_cannot_be_chosen_or_changed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let highest = |categories: &[Category]| {
            categories
                .iter()
                .map(|category| category.id.get())
                .max()
                .unwrap_or(0)
        };
        let before = highest(&stored(database).await);

        let new = json!({"category": "Books", "image": "i", "id": before + 40});
        let added = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let books = before + 1;
        assert_eq!(highest(&stored(database).await), books);

        let path = format!("/categories/{books}");
        let body = json!({"id": books + 40, "order": 77});
        let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let categories = stored(database).await;
        assert_eq!(highest(&categories), books);
        let kept = categories
            .iter()
            .find(|category| category.id.get() == books)
            .expect("the category kept its id");
        assert_eq!(
            kept.position, 77,
            "the other field of the request was stored"
        );
    })
    .await;
}

#[tokio::test]
async fn new_default_category_takes_the_mark_from_the_other_categories() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        // The category "Unsorted" of the seed is the default.
        let new = json!({"category": "Books", "image": "i", "default": 1});
        let answer = call(&app, As::Admin, "POST", "/categories", Some(new)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let defaults: Vec<String> = stored(database)
            .await
            .into_iter()
            .filter(|category| category.is_default)
            .map(|category| category.name)
            .collect();
        assert_eq!(defaults, ["Books"]);
    })
    .await;
}

#[tokio::test]
async fn name_is_compared_as_text_not_as_a_pattern() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        store_category(database, 1, 2, "Media").await;

        // PHP compares with `LIKE`, where `_` and `%` stand for any characters, and so
        // takes these names for taken.
        for name in ["M_dia", "M%", "%"] {
            let new = json!({"category": name, "image": "i"});
            let answer = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
            assert_eq!(answer.status, StatusCode::OK, "{name}: {}", answer.body);
        }
        let new = json!({"category": "mEDIA", "image": "i"});
        let answer = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
        assert_refused(
            &answer,
            StatusCode::CONFLICT,
            "Category name: mEDIA is already taken",
            "another letter case",
        );
    })
    .await;
}

#[tokio::test]
async fn two_categories_cannot_share_a_number() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 7, 2, "Media").await;
        let path = format!("/categories/{}", media.id);
        let before = stored(database).await;
        let taken = "Category category_id: 0 is already taken";

        // The category "Unsorted" of the seed has the number 0. PHP stores both.
        let new = json!({"category": "Books", "image": "i", "category_id": 0});
        let added = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
        let body = json!({"category_id": "0", "order": 9});
        let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_refused(&added, StatusCode::CONFLICT, taken, "add");
        assert_refused(&changed, StatusCode::CONFLICT, taken, "change");
        assert_eq!(stored(database).await, before);

        // A category can be given the number it has.
        let body = json!({"category_id": 7, "order": 9});
        let own = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(own.status, StatusCode::OK, "{}", own.body);
        // A new category without a number gets the one after the highest.
        let new = json!({"category": "Books", "image": "i"});
        let added = call(&app, As::Admin, "POST", "/categories", Some(new)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let books = stored(database)
            .await
            .into_iter()
            .find(|category| category.name == "Books")
            .expect("the category was stored");
        assert_eq!(books.number, CategoryNumber::new(8));
        assert_eq!(books.position, 10, "the place after the last category");
    })
    .await;
}

#[tokio::test]
async fn category_with_tabs_is_not_deleted() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 7, 2, "Media").await;
        let path = format!("/categories/{}", media.id);
        let tab = database
            .tabs()
            .list()
            .await
            .expect("the tabs can be read")
            .into_iter()
            .next()
            .expect("the seed has a tab");
        let tab_path = format!("/tabs/{}", tab.id);
        let moved = call(
            &app,
            As::Admin,
            "PUT",
            &tab_path,
            Some(json!({"category_id": 7})),
        )
        .await;
        assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);

        // PHP deletes the category and leaves the tab without one.
        let refused = call(&app, As::Admin, "DELETE", &path, None).await;
        assert_refused(
            &refused,
            StatusCode::CONFLICT,
            "Category name: Media still has tabs",
            "a category with a tab",
        );
        assert!(stored(database).await.contains(&media));

        // Without the tab, the category can be deleted.
        let moved = call(
            &app,
            As::Admin,
            "PUT",
            &tab_path,
            Some(json!({"category_id": 0})),
        )
        .await;
        assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
        let deleted = call(&app, As::Admin, "DELETE", &path, None).await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT, "{}", deleted.body);
        assert!(!stored(database).await.contains(&media));
    })
    .await;
}

#[tokio::test]
async fn new_tab_needs_a_category_when_none_is_the_default() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let unsorted = stored(database).await[0].clone();
        let path = format!("/categories/{}", unsorted.id);
        let body = json!({"default": 0});
        let changed = call(&app, As::Admin, "PUT", &path, Some(body)).await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let tab = |fields: Value| {
            let mut body =
                json!({"name": "Plex", "url": "https://plex.example.com/", "image": "i"});
            for (key, value) in fields.as_object().expect("the fields are an object") {
                body[key] = value.clone();
            }
            body
        };

        // PHP stores the tab without a category.
        let refused = call(&app, As::Admin, "POST", "/tabs", Some(tab(json!({})))).await;
        assert_refused(
            &refused,
            StatusCode::UNPROCESSABLE_ENTITY,
            "Tab category_id is not valid",
            "no default category",
        );
        let named = tab(json!({"category_id": 0}));
        let added = call(&app, As::Admin, "POST", "/tabs", Some(named)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
    })
    .await;
}

#[tokio::test]
async fn one_malformed_entry_changes_no_place() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let media = store_category(database, 1, 2, "Media").await;
        let before = stored(database).await;
        let first = before[0].id.get();
        let second = media.id.get();

        // PHP stores the entries before the malformed one.
        for malformed in [
            json!({"id": second}),
            json!({"id": second, "order": 0}),
            json!({"id": second, "order": 8, "name": "x"}),
            json!(5),
        ] {
            let body = json!([{"id": first, "order": 70}, malformed]);
            let answer = call(&app, As::Admin, "PUT", "/categories", Some(body.clone())).await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                "data is malformed",
                &body.to_string(),
            );
        }
        // Requests that make PHP crash: no body, and one entry outside a list.
        let without_body = call(&app, As::Admin, "PUT", "/categories", None).await;
        assert_refused(
            &without_body,
            StatusCode::UNPROCESSABLE_ENTITY,
            "data is empty or not in array",
            "no body",
        );
        let body = json!({"id": first, "order": 13});
        let flat = call(&app, As::Admin, "PUT", "/categories", Some(body)).await;
        assert_refused(
            &flat,
            StatusCode::UNPROCESSABLE_ENTITY,
            "data is malformed",
            "one entry outside a list",
        );
        assert_eq!(stored(database).await, before);
    })
    .await;
}
