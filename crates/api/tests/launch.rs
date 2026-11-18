//! `GET /launch` through the real route table: what the recorded PHP answers cannot show.
//!
//! The recording of a fresh install has one tab that an admin sees and none for a guest.
//! These tests give an install more tabs and look at who sees which, in which order, and
//! at the settings that change how the page looks.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{
    CategoryNumber, GroupLevel, LevelRange, NewCategory, NewTab, NewUser, TabKind, Timestamp,
};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{installed_app, installed_app_with, seed};
use serde_json::json;

const USER_PASSWORD: &str = "the password of zoe";

fn level(value: u16) -> GroupLevel {
    GroupLevel::new(value).expect("the level is valid")
}

/// Logs in and returns the session cookie as a browser sends it back.
async fn log_in(app: &Router, name: &str, password: &str) -> String {
    let body = json!({"username": name, "password": password}).to_string();
    let headers = [("content-type", "application/json")];
    let answer = send(app, request("POST", "/api/v2/login", &headers, &body)).await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_"))
        .and_then(|cookie| cookie.split(';').next())
        .unwrap_or_else(|| panic!("the login sets the session cookie: {}", answer.body))
        .to_owned()
}

async fn launch(app: &Router, cookie: &str) -> Answer {
    let headers: Vec<(&str, &str)> = if cookie.is_empty() {
        Vec::new()
    } else {
        vec![("cookie", cookie)]
    };
    send(app, request("GET", "/api/v2/launch", &headers, "")).await
}

/// The names of the tabs in a launch answer, in order.
fn tab_names(answer: &Answer) -> Vec<String> {
    answer.data()["tabs"]
        .as_array()
        .expect("the tabs are a list")
        .iter()
        .map(|tab| tab["name"].as_str().expect("a tab has a name").to_owned())
        .collect()
}

/// Gives the seeded install a second category, a user of level 4, and four more tabs.
async fn add_tabs_and_a_user(database: &Database) {
    let unsorted = database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
        .into_iter()
        .next()
        .expect("the seed has a category");
    let media = database
        .categories()
        .insert(NewCategory {
            number: CategoryNumber::new(7),
            position: 2,
            name: "Media".to_owned(),
            image: None,
            is_default: false,
        })
        .await
        .expect("the category can be stored");

    let tab = |position: i64, name: &str, category, least_access: u16, most_access: u16| NewTab {
        position,
        category,
        name: name.to_owned(),
        url: Some(format!("https://{}.example.com/", name.to_lowercase())),
        url_local: None,
        is_default: false,
        enabled: true,
        access: LevelRange::new(level(least_access), level(most_access))
            .expect("the range is valid"),
        add_to_admin: false,
        image: Some("plugins/images/tabs/plex.png".to_owned()),
        kind: TabKind::Iframe,
        splash: true,
        ping: false,
        ping_url: None,
        timeout: false,
        timeout_ms: None,
        preload: false,
    };
    let tabs = [
        // For every level from Admin to User.
        tab(3, "Plex", media.id, 4, 0),
        // For everyone, guests too.
        tab(4, "Status", unsorted.id, 999, 0),
        // For guests only, and through "add to admin" also for admins.
        NewTab {
            add_to_admin: true,
            ..tab(5, "Welcome", unsorted.id, 999, 999)
        },
        // Switched off.
        NewTab {
            enabled: false,
            ..tab(6, "Old", media.id, 999, 0)
        },
    ];
    for new in tabs {
        database
            .tabs()
            .insert(new)
            .await
            .expect("the tab can be stored");
    }

    let group = database
        .groups()
        .find_by_level(level(4))
        .await
        .expect("the lookup works")
        .expect("the seed has the group User");
    database
        .users()
        .insert(NewUser {
            username: "zoe".to_owned(),
            email: None,
            password_hash: password::hash(USER_PASSWORD).expect("the password can be hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
}

#[tokio::test]
async fn each_caller_gets_the_tabs_of_their_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        let app = installed_app(database);
        let admin = log_in(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let zoe = log_in(&app, "zoe", USER_PASSWORD).await;

        // The default order is downwards by the place of a tab.
        assert_eq!(tab_names(&launch(&app, "").await), ["Welcome", "Status"]);
        assert_eq!(tab_names(&launch(&app, &zoe).await), ["Status", "Plex"]);
        // The admin sees "Welcome" because it is marked "add to admin". The tabs
        // "Homepage" of the set-up and "Old" are switched off.
        assert_eq!(
            tab_names(&launch(&app, &admin).await),
            ["Welcome", "Status", "Plex", "Settings"]
        );
    })
    .await;
}

#[tokio::test]
async fn categories_count_the_tabs_that_the_caller_sees() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        let app = installed_app(database);
        let zoe = log_in(&app, "zoe", USER_PASSWORD).await;
        let counts = |answer: &Answer| -> Vec<(String, i64, i64)> {
            answer.data()["categories"]
                .as_array()
                .expect("the categories are a list")
                .iter()
                .map(|category| {
                    (
                        category["category"]
                            .as_str()
                            .expect("a category has a name")
                            .to_owned(),
                        category["category_id"].as_i64().expect("a number"),
                        category["count"].as_i64().expect("a count"),
                    )
                })
                .collect()
        };

        // Name, number and count of each category, in menu order.
        assert_eq!(
            counts(&launch(&app, "").await),
            [("Unsorted".to_owned(), 0, 2), ("Media".to_owned(), 7, 0)]
        );
        assert_eq!(
            counts(&launch(&app, &zoe).await),
            [("Unsorted".to_owned(), 0, 1), ("Media".to_owned(), 7, 1)]
        );
    })
    .await;
}

#[tokio::test]
async fn tab_is_sent_with_the_fields_of_the_php_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        let app = installed_app(database);
        let zoe = log_in(&app, "zoe", USER_PASSWORD).await;

        let answer = launch(&app, &zoe).await;

        let plex = answer.data()["tabs"][1].clone();
        let id = plex["id"].clone();
        assert_eq!(
            plex,
            json!({
                "id": id,
                "order": 3,
                // The number of the category "Media", not its id.
                "category_id": 7,
                "name": "Plex",
                "url": "https://plex.example.com/",
                "url_local": null,
                "default": 0,
                "enabled": 1,
                "group_id": 4,
                "group_id_max": 0,
                "add_to_admin": 0,
                "image": "plugins/images/tabs/plex.png",
                "type": 1,
                "splash": 1,
                "ping": 0,
                "ping_url": null,
                "timeout": 0,
                "timeout_ms": null,
                "preload": 0,
                "access_url": "https://plex.example.com/",
            })
        );
        assert_eq!(answer.data()["user"]["username"], json!("zoe"));
        assert_eq!(answer.data()["status"]["status"], json!("ok"));
    })
    .await;
}

#[tokio::test]
async fn settings_change_the_order_and_the_look() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        let app = installed_app_with(
            database,
            &[
                ("unsortedTabs", json!("bottom")),
                ("title", json!("Home")),
                ("theme", json!("Plex")),
                ("style", json!("light")),
                ("useLogo", json!(true)),
                ("logo", json!("https://example.com/logo.png")),
                ("headerColor", json!("#112233")),
                ("customCss", json!("body { margin: 0; }")),
            ],
        );

        let answer = launch(&app, "").await;

        let data = answer.data();
        // Upwards by the place of a tab.
        assert_eq!(tab_names(&answer), ["Status", "Welcome"]);
        assert_eq!(data["theme"], json!("Plex"));
        assert_eq!(data["style"], json!("light"));
        assert_eq!(data["appearance"]["title"], json!("Home"));
        assert_eq!(data["appearance"]["useLogo"], json!(true));
        assert_eq!(
            data["appearance"]["logo"],
            json!("https://example.com/logo.png")
        );
        assert_eq!(data["appearance"]["headerColor"], json!("#112233"));
        assert_eq!(
            data["appearance"]["customCss"],
            json!("body { margin: 0; }")
        );
        // What was not changed has the default of the PHP application.
        assert_eq!(data["appearance"]["useLogoLogin"], json!(false));
        assert_eq!(
            data["appearance"]["loginLogo"],
            json!("plugins/images/organizr/organizr-logo-h.png")
        );
    })
    .await;
}

#[tokio::test]
async fn caller_with_the_api_key_gets_the_menu_of_an_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        let app = installed_app(database);

        let answer = send(
            &app,
            request("GET", "/api/v2/launch", &[("token", seed::API_KEY)], ""),
        )
        .await;

        assert_eq!(answer.data()["user"]["username"], json!("Organizr API"));
        assert_eq!(
            tab_names(&answer),
            ["Welcome", "Status", "Plex", "Settings"]
        );
    })
    .await;
}
