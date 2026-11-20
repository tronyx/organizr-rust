//! The routes of the bookmark plugin.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const TABS: &str = "/plugins/bookmark/tabs";
const CATEGORIES: &str = "/plugins/bookmark/categories";

fn on() -> Vec<(&'static str, Value)> {
    vec![("BOOKMARK-enabled", json!(true))]
}

fn bookmark(name: &str, more: &Value) -> Value {
    let mut body = json!({
        "name": name,
        "url": format!("https://{}.example.com", name.to_lowercase()),
        "image": "fontawesome::bookmark",
        "background_color": "#808080",
        "text_color": "#fff",
        "enabled": 1,
    });
    for (key, value) in more.as_object().expect("an object") {
        body[key] = value.clone();
    }
    body
}

async fn add(app: &Router, body: Value) -> Answer {
    call(app, As::Admin, "POST", TABS, Some(body)).await
}

async fn listed(app: &Router) -> Value {
    call(app, As::Admin, "GET", TABS, None).await.data()
}

#[tokio::test]
async fn bookmarks_are_added_changed_ordered_and_deleted() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &on());

        let made = call(&app, As::Admin, "GET", "/plugins/bookmark/setup/category", None).await;
        assert_eq!(made.message(), json!("Category created"));
        let again = call(&app, As::Admin, "GET", "/plugins/bookmark/setup/category", None).await;
        assert_eq!(again.message(), json!("Categories already exists"));

        let added = add(&app, bookmark("Wiki", &json!({}))).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        assert_eq!(added.message(), json!("Tab added"));
        add(&app, bookmark("News", &json!({"group_id": 1, "enabled": 0}))).await;

        let all = listed(&app).await;
        assert_eq!(
            all["tabs"][0],
            json!({
                "id": all["tabs"][0]["id"],
                "order": 1,
                // The default category, and the level of the group for new users.
                "category_id": 1,
                "name": "Wiki",
                "url": "https://wiki.example.com",
                "enabled": 1,
                "group_id": 4,
                "image": "fontawesome::bookmark",
                "background_color": "#808080",
                "text_color": "#fff",
            })
        );
        assert_eq!(all["tabs"][1]["order"], 2);
        assert_eq!(all["tabs"][1]["group_id"], 1);
        assert_eq!(all["tabs"][1]["enabled"], 0);
        assert_eq!(
            all["categories"],
            json!([{"id": all["categories"][0]["id"], "order": 1, "category": "Unsorted", "category_id": 1, "default": 1}])
        );
        assert!(all["groups"].as_array().is_some_and(|groups| groups.len() >= 2));
        let (wiki, news) = (all["tabs"][0]["id"].clone(), all["tabs"][1]["id"].clone());

        let changed = call(
            &app,
            As::Admin,
            "PUT",
            &format!("{TABS}/{wiki}"),
            Some(json!({"name": "Docs", "text_color": "#000000"})),
        )
        .await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        assert_eq!(changed.message(), json!("Tab info updated"));
        let order = json!([{"id": wiki, "order": 2}, {"id": news, "order": 1}]);
        let ordered = call(&app, As::Admin, "PUT", TABS, Some(order)).await;
        assert_eq!(ordered.message(), json!("Tab Order updated"));
        let all = listed(&app).await;
        assert_eq!(all["tabs"][0]["name"], "News");
        assert_eq!(all["tabs"][1]["name"], "Docs");
        assert_eq!(all["tabs"][1]["text_color"], "#000000");
        assert_eq!(all["tabs"][1]["url"], "https://wiki.example.com");

        let deleted = call(&app, As::Admin, "DELETE", &format!("{TABS}/{wiki}"), None).await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let missing = call(&app, As::Admin, "DELETE", &format!("{TABS}/{wiki}"), None).await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        assert_eq!(missing.message(), json!("id not found"));
    })
    .await;
}

#[tokio::test]
async fn bookmark_that_is_not_valid_is_refused_with_the_reason() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &on());
        add(&app, bookmark("Wiki", &json!({}))).await;
        let without = |field: &str| {
            let mut body = bookmark("Other", &json!({}));
            body.as_object_mut().expect("an object").remove(field);
            body
        };

        for (body, status, message) in [
            (json!({}), 422, "no data was sent"),
            (without("name"), 422, "Tab name was not supplied"),
            (bookmark("  ", &json!({})), 422, "Tab name was not supplied"),
            (
                bookmark("WIKI", &json!({})),
                409,
                "Tab name: WIKI is already taken",
            ),
            (without("url"), 422, "Tab url was not supplied"),
            (without("image"), 422, "Tab image was not supplied"),
            (
                without("background_color"),
                422,
                "Tab background color was not supplied",
            ),
            (
                without("text_color"),
                422,
                "Tab text color was not supplied",
            ),
            (
                bookmark("Other", &json!({"background_color": "red"})),
                422,
                "Tab background color is invalid",
            ),
            (
                bookmark("Other", &json!({"text_color": "#12345"})),
                422,
                "Tab text color is invalid",
            ),
            (
                bookmark("Other", &json!({"group_id": 1000})),
                422,
                "Tab group_id is not valid",
            ),
        ] {
            let refused = add(&app, body.clone()).await;
            assert_eq!(refused.status.as_u16(), status, "{body}: {}", refused.body);
            assert_eq!(refused.message(), json!(message), "{body}");
        }
        assert_eq!(listed(&app).await["tabs"].as_array().map(Vec::len), Some(1));

        let id = listed(&app).await["tabs"][0]["id"].clone();
        let path = format!("{TABS}/{id}");
        add(&app, bookmark("News", &json!({}))).await;
        for (body, status, message) in [
            (json!({}), 422, "no data was sent"),
            (
                json!({"name": "news"}),
                409,
                "Tab name: news is already taken",
            ),
            (
                json!({"background_color": "#gggggg"}),
                422,
                "Tab background color is invalid",
            ),
        ] {
            let refused = call(&app, As::Admin, "PUT", &path, Some(body.clone())).await;
            assert_eq!(refused.status.as_u16(), status, "{body}: {}", refused.body);
            assert_eq!(refused.message(), json!(message), "{body}");
        }
        // A bookmark can keep its own name.
        let kept = call(&app, As::Admin, "PUT", &path, Some(json!({"name": "wiki"}))).await;
        assert_eq!(kept.status, StatusCode::OK, "{}", kept.body);
        let missing = call(
            &app,
            As::Admin,
            "PUT",
            &format!("{TABS}/9999"),
            Some(json!({"name": "A"})),
        )
        .await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        assert_eq!(missing.message(), json!("No tab info found"));
        let wrong = call(&app, As::Admin, "PUT", TABS, Some(json!([{"id": id}]))).await;
        assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(wrong.message(), json!("data is malformed"));
    })
    .await;
}

#[tokio::test]
async fn categories_keep_one_default_and_their_bookmarks_stay_when_they_go() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &on());
        let post = async |body: Value| call(&app, As::Admin, "POST", CATEGORIES, Some(body)).await;

        let added = post(json!({"category": "Work"})).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        assert_eq!(added.message(), json!("Category added"));
        post(json!({"category": "Home", "default": 1})).await;
        let taken = post(json!({"category": "work"})).await;
        assert_eq!(taken.status, StatusCode::CONFLICT);
        assert_eq!(
            taken.message(),
            json!("Category name: work is already taken")
        );
        let taken = post(json!({"category": "Other", "category_id": 2})).await;
        assert_eq!(
            taken.message(),
            json!("Category category_id: 2 is already taken")
        );
        let empty = post(json!({"default": 1})).await;
        assert_eq!(empty.message(), json!("Category name was not supplied"));

        let all = call(&app, As::Admin, "GET", CATEGORIES, None).await.data();
        let rows = all["categories"].as_array().expect("a list").clone();
        let defaults: Vec<&str> = rows
            .iter()
            .filter(|row| row["default"] == 1)
            .map(|row| row["category"].as_str().unwrap_or("?"))
            .collect();
        assert_eq!(defaults, ["Home"]);
        let (work, home) = (rows[0]["id"].clone(), rows[1]["id"].clone());

        // A new bookmark joins the default category.
        add(&app, bookmark("Wiki", &json!({}))).await;
        assert_eq!(listed(&app).await["tabs"][0]["category_id"], 2);

        let changed = call(
            &app,
            As::Admin,
            "PUT",
            &format!("{CATEGORIES}/{work}"),
            Some(json!({"category": "Office", "default": 1})),
        )
        .await;
        assert_eq!(changed.message(), json!("Category info updated"));
        let order = json!([{"id": work, "order": 5}, {"id": home, "order": 1}]);
        let ordered = call(&app, As::Admin, "PUT", CATEGORIES, Some(order)).await;
        assert_eq!(ordered.message(), json!("Category Order updated"));
        let rows = listed(&app).await["categories"].clone();
        assert_eq!(rows[0]["category"], "Home");
        assert_eq!(rows[0]["default"], 0);
        assert_eq!(rows[1]["category"], "Office");
        assert_eq!(rows[1]["default"], 1);

        // The default category goes: the other one is the default, and the bookmark
        // of the category that is gone stays.
        let deleted = call(
            &app,
            As::Admin,
            "DELETE",
            &format!("{CATEGORIES}/{work}"),
            None,
        )
        .await;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
        let all = listed(&app).await;
        assert_eq!(all["categories"][0]["default"], 1);
        assert_eq!(all["tabs"].as_array().map(Vec::len), Some(1));
        let missing = call(
            &app,
            As::Admin,
            "DELETE",
            &format!("{CATEGORIES}/{work}"),
            None,
        )
        .await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn page_shows_each_caller_the_bookmarks_of_the_own_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &on());
        let user = seed::logged_in(database, &app, "zed", 4).await;
        call(
            &app,
            As::Admin,
            "POST",
            CATEGORIES,
            Some(json!({"category": "Work"})),
        )
        .await;
        call(
            &app,
            As::Admin,
            "POST",
            CATEGORIES,
            Some(json!({"category": "Empty"})),
        )
        .await;
        add(
            &app,
            bookmark("Wiki", &json!({"group_id": 999, "category_id": 1})),
        )
        .await;
        add(
            &app,
            bookmark("Admin", &json!({"group_id": 1, "category_id": 1})),
        )
        .await;
        add(
            &app,
            bookmark(
                "Off",
                &json!({"group_id": 999, "category_id": 1, "enabled": 0}),
            ),
        )
        .await;
        add(
            &app,
            bookmark("Lost", &json!({"group_id": 999, "category_id": 77})),
        )
        .await;
        let names = |answer: &Answer| -> Vec<String> {
            answer.data()["categories"]
                .as_array()
                .expect("a list")
                .iter()
                .flat_map(|category| category["tabs"].as_array().cloned().unwrap_or_default())
                .map(|tab| tab["name"].as_str().unwrap_or("?").to_owned())
                .collect()
        };
        let path = "/plugins/bookmark/page";

        let admin = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(admin.status, StatusCode::OK, "{}", admin.body);
        assert_eq!(names(&admin), ["Wiki", "Admin"]);
        // The category without bookmarks is left out.
        assert_eq!(admin.data()["categories"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            admin.data()["categories"][0]["tabs"][0],
            json!({
                "name": "Wiki",
                "url": "https://wiki.example.com",
                "image": "fontawesome::bookmark",
                "background_color": "#808080",
                "text_color": "#fff",
                "border_color": "#a7a7a7",
                "dark_color": "#5a5a5a",
                "light_color": "#8d8d8d",
            })
        );
        // Without a tab of the menu for the page, everybody sees it.
        assert_eq!(
            names(&call(&app, As::User(&user), "GET", path, None).await),
            ["Wiki"]
        );
        assert_eq!(
            names(&call(&app, As::Guest, "GET", path, None).await),
            ["Wiki"]
        );

        // The tab of the menu is for the group of new users, so a guest is refused.
        let made = call(&app, As::Admin, "GET", "/plugins/bookmark/setup/tab", None).await;
        assert_eq!(made.message(), json!("Tab created"));
        let again = call(&app, As::Admin, "GET", "/plugins/bookmark/setup/tab", None).await;
        assert_eq!(again.message(), json!("Tab already exists"));
        assert_eq!(
            call(&app, As::Guest, "GET", path, None).await.status,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            names(&call(&app, As::User(&user), "GET", path, None).await),
            ["Wiki"]
        );

        // One bookmark is for callers who have its level.
        let id = listed(&app).await["tabs"][1]["id"].clone();
        let refused = call(&app, As::User(&user), "GET", &format!("{TABS}/{id}"), None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let shown = call(&app, As::Admin, "GET", &format!("{TABS}/{id}"), None).await;
        assert_eq!(shown.data()["name"], "Admin");
    })
    .await;
}

#[tokio::test]
async fn routes_need_the_plugin_to_be_on_and_the_editor_needs_an_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let off = installed_app_with(database, &[]);
        for path in [
            "/plugins/bookmark/page",
            TABS,
            CATEGORIES,
            "/plugins/bookmark/setup/tab",
        ] {
            let refused = call(&off, As::Admin, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::CONFLICT, "{path}");
            assert_eq!(
                refused.message(),
                json!("BOOKMARK-enabled module is not enabled")
            );
        }

        let app = installed_app_with(database, &on());
        let user = seed::logged_in(database, &app, "zed", 4).await;
        for (method, path) in [
            ("GET", TABS.to_owned()),
            ("POST", TABS.to_owned()),
            ("PUT", TABS.to_owned()),
            ("PUT", format!("{TABS}/1")),
            ("DELETE", format!("{TABS}/1")),
            ("GET", CATEGORIES.to_owned()),
            ("POST", CATEGORIES.to_owned()),
            ("PUT", CATEGORIES.to_owned()),
            ("PUT", format!("{CATEGORIES}/1")),
            ("DELETE", format!("{CATEGORIES}/1")),
            ("GET", "/plugins/bookmark/setup/tab".to_owned()),
            ("GET", "/plugins/bookmark/setup/category".to_owned()),
        ] {
            let body = (method != "GET" && method != "DELETE").then(|| json!({"name": "A"}));
            let refused = call(&app, As::User(&user), method, &path, body).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{method} {path}");
        }
    })
    .await;
}
