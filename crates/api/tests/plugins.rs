//! The plugin manager routes: the lists of the built-in plugins and the plugin
//! marketplace (D-94).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// The names of the plugins of a list, in its order.
fn names(data: &Value) -> Vec<String> {
    data.as_object()
        .expect("the list is an object")
        .keys()
        .cloned()
        .collect()
}

#[tokio::test]
async fn everybody_gets_the_built_in_plugins_with_the_fields_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("CHAT-enabled", json!(true)),
                ("BOOKMARK-enabled", json!(true)),
            ],
        );
        let answer = call(&app, As::Guest, "GET", "/plugins", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(
            names(&data),
            [
                "Chat",
                "HealthChecks",
                "Invites",
                "PHP Mailer",
                "SpeedTest",
                "Bookmark",
                // The plugin of this application comes last (D-121).
                "Plex Libraries"
            ]
        );
        assert!(!answer.body.to_lowercase().contains("shuck"));
        assert_eq!(
            data["Chat"],
            json!({
                "name": "Chat",
                "author": "CauseFX",
                "category": "Utilities",
                "link": "",
                "license": "personal,business",
                "idPrefix": "CHAT",
                "configPrefix": "CHAT",
                "version": "1.0.0",
                "image": "api/plugins/chat/logo.png",
                "settings": true,
                "bind": true,
                "api": "api/v2/plugins/chat/settings",
                "homepage": false,
                "enabled": true,
            })
        );
        assert_eq!(data["HealthChecks"]["api"], json!(false));
        assert_eq!(data["Bookmark"]["dbPrefix"], json!("BOOKMARK"));

        let enabled = call(&app, As::Guest, "GET", "/plugins/enabled", None).await;
        assert_eq!(names(&enabled.data()), ["Chat", "Bookmark"]);
        let disabled = call(&app, As::Guest, "GET", "/plugins/disabled", None).await;
        assert_eq!(
            names(&disabled.data()),
            [
                "HealthChecks",
                "Invites",
                "PHP Mailer",
                "SpeedTest",
                "Plex Libraries"
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn the_license_of_the_install_picks_the_plugins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("license", json!("business"))]);
        let answer = call(&app, As::Guest, "GET", "/plugins", None).await;
        assert!(!names(&answer.data()).contains(&"Invites".to_owned()));
    })
    .await;
}

#[tokio::test]
async fn the_marketplace_is_for_admins_and_lists_no_php_plugin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        for (method, path) in [
            ("GET", "/plugins/marketplace"),
            ("POST", "/plugins/manage/plex_libraries"),
            ("DELETE", "/plugins/manage/plex_libraries"),
        ] {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            }
        }
        let answer = call(
            &app,
            As::User(&co_admin),
            "GET",
            "/plugins/marketplace",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), json!({}));
        for method in ["POST", "DELETE"] {
            let answer = call(
                &app,
                As::Admin,
                method,
                "/plugins/manage/plex_libraries",
                None,
            )
            .await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{}", answer.body);
            assert_eq!(
                answer.message(),
                json!("Plugin does not exist in marketplace")
            );
        }
    })
    .await;
}

#[tokio::test]
async fn shuck_stop_routes_are_unknown() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // Shuck Stop was removed with its settings (D-116).
        let app = installed_app_with(database, &[]);
        for path in ["/plugins/shuck-stop/settings", "/plugins/shuck-stop/run"] {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
        }
    })
    .await;
}
