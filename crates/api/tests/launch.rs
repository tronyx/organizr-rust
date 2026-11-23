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

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{
    CategoryNumber, GroupLevel, LevelRange, NewCategory, NewTab, NewUser, TabKind, Timestamp,
};
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{installed_app, installed_app_with, seed, state_with};
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
                // A tab without a local address has the empty text there, as in PHP.
                "url_local": "",
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
                // The page shows a theme of the marketplace only when it is installed (D-92).
                (
                    "installedThemes",
                    json!({ "Plex": { "name": "Plex", "version": "3.0.0" } }),
                ),
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

/// A JPEG header of the given width, enough for the width to be read.
fn jpeg_of_width(width: u16) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
    bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x02, 0xA3]);
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&[0x03; 12]);
    bytes
}

#[tokio::test]
async fn login_page_gets_a_wide_picture_of_the_cache_when_the_setting_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let cache = folder.path().join("cache");
        std::fs::create_dir_all(&cache).expect("the cache folder can be made");
        // Only the picture of what plays that is wide enough can be chosen.
        std::fs::write(cache.join("tautulli-7-np.jpg"), jpeg_of_width(2400)).expect("written");
        std::fs::write(cache.join("tautulli-8-np.jpg"), jpeg_of_width(600)).expect("written");
        std::fs::write(cache.join("tautulli-7-list.jpg"), jpeg_of_width(2400)).expect("written");
        let app_for = |on: bool| {
            let settings =
                organizr_testkit::installed_settings(&[("useRandomMediaImage", json!(on))]);
            organizr_api::router(state_with(database, settings).with_picture_cache(cache.clone()))
                .expect("the route table is valid")
        };

        let answer = launch(&app_for(true), "").await;
        assert_eq!(
            answer.data()["appearance"]["randomMediaImage"],
            json!("data/cache/tautulli-7-np.jpg")
        );
        let off = launch(&app_for(false), "").await;
        assert_eq!(off.data()["appearance"]["randomMediaImage"], json!(false));

        // Without a picture that is wide enough, there is none.
        std::fs::remove_file(cache.join("tautulli-7-np.jpg")).expect("removed");
        let none = launch(&app_for(true), "").await;
        assert_eq!(none.data()["appearance"]["randomMediaImage"], json!(false));
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

/// The address of a caller on a home network, and one on the internet.
const AT_HOME: &str = "192.168.1.20:50000";
const ELSEWHERE: &str = "203.0.113.7:50000";

/// The launch answer for the script with the API key, sent from the given address of a
/// connection, with more headers.
async fn launch_from(app: &Router, peer: &str, more_headers: &[(&str, &str)]) -> Answer {
    let mut headers = vec![("token", seed::API_KEY)];
    headers.extend_from_slice(more_headers);
    let mut request = request("GET", "/api/v2/launch", &headers, "");
    let peer: SocketAddr = peer.parse().expect("the address is valid");
    request.extensions_mut().insert(ConnectInfo(peer));
    send(app, request).await
}

/// The tab with the given name in a launch answer.
fn tab_named(answer: &Answer, name: &str) -> serde_json::Value {
    answer.data()["tabs"]
        .as_array()
        .expect("the tabs are a list")
        .iter()
        .find(|tab| tab["name"] == json!(name))
        .cloned()
        .unwrap_or_else(|| panic!("the answer has no tab `{name}`: {}", answer.body))
}

/// Stores an enabled tab for every level, with the two addresses.
async fn store_tab(database: &Database, name: &str, url: Option<&str>, url_local: Option<&str>) {
    let category = database
        .categories()
        .list()
        .await
        .expect("the categories can be read")
        .into_iter()
        .next()
        .expect("the seed has a category");
    database
        .tabs()
        .insert(NewTab {
            position: 10,
            category: category.id,
            name: name.to_owned(),
            url: url.map(str::to_owned),
            url_local: url_local.map(str::to_owned),
            is_default: false,
            enabled: true,
            access: LevelRange::up_to(GroupLevel::GUEST),
            add_to_admin: false,
            image: None,
            kind: TabKind::Iframe,
            splash: false,
            ping: false,
            ping_url: None,
            timeout: false,
            timeout_ms: None,
            preload: false,
        })
        .await
        .expect("the tab can be stored");
}

#[tokio::test]
async fn caller_on_the_local_network_opens_the_local_address_of_a_tab() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_tab(
            database,
            "Plex",
            Some("https://plex.example.com/"),
            Some("http://10.0.0.6:32400/web"),
        )
        .await;
        store_tab(
            database,
            "Sonarr",
            Some("https://sonarr.example.com/"),
            None,
        )
        .await;
        let app = installed_app(database);

        let at_home = launch_from(&app, AT_HOME, &[]).await;
        let elsewhere = launch_from(&app, ELSEWHERE, &[]).await;

        // Both callers get both addresses. Which one to open depends on the caller.
        for answer in [&at_home, &elsewhere] {
            let plex = tab_named(answer, "Plex");
            assert_eq!(plex["url"], json!("https://plex.example.com/"));
            assert_eq!(plex["url_local"], json!("http://10.0.0.6:32400/web"));
        }
        assert_eq!(
            tab_named(&at_home, "Plex")["access_url"],
            json!("http://10.0.0.6:32400/web")
        );
        assert_eq!(
            tab_named(&elsewhere, "Plex")["access_url"],
            json!("https://plex.example.com/")
        );
        // A tab without a local address opens its address for everyone.
        assert_eq!(
            tab_named(&at_home, "Sonarr")["access_url"],
            json!("https://sonarr.example.com/")
        );
    })
    .await;
}

#[tokio::test]
async fn setting_names_more_local_networks() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_tab(
            database,
            "Plex",
            Some("https://plex.example.com/"),
            Some("http://10.0.0.6/"),
        )
        .await;
        let app = installed_app_with(
            database,
            &[("localIPList", json!("198.51.100.4, 203.0.113.0/24"))],
        );

        let listed = launch_from(&app, ELSEWHERE, &[]).await;
        let not_listed = launch_from(&app, "198.51.100.5:50000", &[]).await;

        assert_eq!(
            tab_named(&listed, "Plex")["access_url"],
            json!("http://10.0.0.6/")
        );
        assert_eq!(
            tab_named(&not_listed, "Plex")["access_url"],
            json!("https://plex.example.com/")
        );
    })
    .await;
}

#[tokio::test]
async fn caller_behind_a_proxy_that_is_not_trusted_is_not_local() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_tab(
            database,
            "Plex",
            Some("https://plex.example.com/"),
            Some("http://10.0.0.6/"),
        )
        .await;
        let public = "https://plex.example.com/";
        let local = "http://10.0.0.6/";

        // The connection comes from a proxy on the home network. Without the proxy in
        // the list of trusted proxies, the true caller is not known.
        let app = installed_app(database);
        let forwarded = [("x-forwarded-for", "192.168.1.30")];
        let answer = launch_from(&app, AT_HOME, &forwarded).await;
        assert_eq!(tab_named(&answer, "Plex")["access_url"], json!(public));

        // With the proxy trusted, the address that it reports counts.
        let mut settings = Settings::defaults();
        settings
            .set("organizrAPI", &json!(seed::API_KEY))
            .expect("the API key is a valid setting");
        let state = state_with(database, settings)
            .with_trusted_proxies("192.168.1.20".parse().expect("the list is valid"));
        let app = organizr_api::router(state).expect("the route table is valid");
        let answer = launch_from(&app, AT_HOME, &forwarded).await;
        assert_eq!(tab_named(&answer, "Plex")["access_url"], json!(local));
        let from_outside = [("x-forwarded-for", "203.0.113.7")];
        let answer = launch_from(&app, AT_HOME, &from_outside).await;
        assert_eq!(tab_named(&answer, "Plex")["access_url"], json!(public));
    })
    .await;
}

#[tokio::test]
async fn addresses_of_a_tab_are_completed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_tab(
            database,
            "Path",
            Some("/plex/{username_lower}"),
            Some("null"),
        )
        .await;
        store_tab(
            database,
            "Bare",
            Some("plex.example.com:32400/web"),
            Some(""),
        )
        .await;
        store_tab(
            database,
            "Secret",
            Some("https://user:secret@plex.example.com/"),
            None,
        )
        .await;
        store_tab(database, "Empty", None, None).await;
        let mut settings = Settings::defaults();
        settings
            .set("organizrAPI", &json!(seed::API_KEY))
            .expect("the API key is a valid setting");
        let state = state_with(database, settings)
            .with_trusted_proxies("192.168.1.20".parse().expect("the list is valid"));
        let app = organizr_api::router(state).expect("the route table is valid");
        let addresses = |answer: &Answer, name: &str| {
            let tab = tab_named(answer, name);
            (
                tab["url"].clone(),
                tab["url_local"].clone(),
                tab["access_url"].clone(),
            )
        };

        let answer = launch_from(&app, AT_HOME, &[]).await;

        // A path gets the host of the request in front, after the placeholders.
        let path = "http://organizr.example.com/plex/organizr api";
        assert_eq!(
            addresses(&answer, "Path"),
            (json!(path), json!(""), json!(path))
        );
        // An address without a scheme gets `http://`.
        let bare = "http://plex.example.com:32400/web";
        assert_eq!(
            addresses(&answer, "Bare"),
            (json!(bare), json!(""), json!(bare))
        );
        // A user name and a password do not reach the browser.
        let secret = "https://plex.example.com/";
        assert_eq!(
            addresses(&answer, "Secret"),
            (json!(secret), json!(""), json!(secret))
        );
        assert_eq!(
            addresses(&answer, "Empty"),
            (json!(""), json!(""), json!(""))
        );
        // The tab "Settings" is a page of Organizr itself. Its addresses stay as they are.
        assert_eq!(
            addresses(&answer, "Settings"),
            (
                json!("api/v2/page/settings"),
                serde_json::Value::Null,
                json!("api/v2/page/settings")
            )
        );

        // Behind a trusted proxy that reports HTTPS, a path gets `https://`.
        let over_https = [
            ("x-forwarded-for", "192.168.1.30"),
            ("x-forwarded-proto", "https"),
        ];
        let answer = launch_from(&app, AT_HOME, &over_https).await;
        assert_eq!(
            tab_named(&answer, "Path")["url"],
            json!("https://organizr.example.com/plex/organizr api")
        );
    })
    .await;
}

#[tokio::test]
async fn placeholders_are_filled_in_for_the_user_who_asks() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        add_tabs_and_a_user(database).await;
        store_tab(
            database,
            "Mine",
            Some("https://{domain}/{username}/{group}/{group_id}"),
            None,
        )
        .await;
        let app = installed_app(database);
        let zoe = log_in(&app, "zoe", USER_PASSWORD).await;

        let for_zoe = launch(&app, &zoe).await;
        let for_a_guest = launch(&app, "").await;

        assert_eq!(
            tab_named(&for_zoe, "Mine")["url"],
            json!("https://organizr.example.com/zoe/User/4")
        );
        assert_eq!(
            tab_named(&for_a_guest, "Mine")["url"],
            json!("https://organizr.example.com/Guest/Guest/999")
        );
    })
    .await;
}
