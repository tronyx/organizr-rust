//! The API key, through the real route table: what the recorded PHP answers cannot show.
//!
//! `crates/testkit/tests/parity.rs` replays how PHP answers a guest with the key. These
//! tests look at a logged-in user with the key, at an install without a key, and at how
//! the key and the same-origin check work together.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_auth::password;
use organizr_db::Database;
use organizr_domain::{GroupLevel, NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{app, installed_app, seed};
use serde_json::json;

const USER_PASSWORD: &str = "the password of zoe";

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

/// Stores a second account, in the group "User" (level 4).
async fn insert_zoe(database: &Database) {
    let group = database
        .groups()
        .find_by_level(GroupLevel::new(4).expect("the level is valid"))
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

async fn me(app: &Router, headers: &[(&str, &str)]) -> Answer {
    send(app, request("GET", "/api/v2/token/me", headers, "")).await
}

#[tokio::test]
async fn logged_in_user_who_sends_the_key_is_still_that_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        insert_zoe(database).await;
        let app = installed_app(database);
        let admin = log_in(&app, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let zoe = log_in(&app, "zoe", USER_PASSWORD).await;

        for (cookie, name, level) in [(&admin, "admin", 0), (&zoe, "zoe", 4)] {
            let answer = me(&app, &[("cookie", cookie), ("token", seed::API_KEY)]).await;

            let data = answer.data();
            assert_eq!(data["username"], json!(name));
            assert_eq!(data["groupID"], json!(level));
            assert_eq!(data["loggedin"], json!(true));
        }
    })
    .await;
}

#[tokio::test]
async fn install_without_a_key_takes_no_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // Every setting at its default: the API key is empty.
        let app = app(database);
        let twenty_spaces = " ".repeat(20);

        for key in ["", twenty_spaces.as_str(), seed::API_KEY] {
            let answer = me(&app, &[("token", key)]).await;
            assert_eq!(answer.data()["username"], json!("Guest"), "{key:?}");

            let revoke = send(
                &app,
                request("DELETE", "/api/v2/token/1", &[("token", key)], ""),
            )
            .await;
            assert_eq!(revoke.status, 401, "{key:?}");
        }
        let in_query = send(&app, request("GET", "/api/v2/token/me?apikey=", &[], "")).await;
        assert_eq!(in_query.data()["username"], json!("Guest"));
    })
    .await;
}

#[tokio::test]
async fn key_lets_a_script_use_a_get_route_that_changes_something() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        // What a browser sends for a request that another site started.
        let cross_site = ("sec-fetch-site", "cross-site");

        let without_key = send(&app, request("GET", "/api/v2/logout", &[cross_site], "")).await;
        let wrong_key = send(
            &app,
            request(
                "GET",
                "/api/v2/logout",
                &[cross_site, ("token", "parityapikey01234568")],
                "",
            ),
        )
        .await;
        let with_key = send(
            &app,
            request(
                "GET",
                "/api/v2/logout",
                &[cross_site, ("token", seed::API_KEY)],
                "",
            ),
        )
        .await;
        let key_in_query = send(
            &app,
            request(
                "GET",
                &format!("/api/v2/logout?apikey={}", seed::API_KEY),
                &[cross_site],
                "",
            ),
        )
        .await;

        // Another site cannot make a browser send the key, so the key is enough.
        assert_eq!(without_key.status, 401, "{}", without_key.body);
        assert_eq!(wrong_key.status, 401, "{}", wrong_key.body);
        assert_eq!(with_key.status, 200, "{}", with_key.body);
        assert_eq!(key_in_query.status, 200, "{}", key_in_query.body);
    })
    .await;
}

#[tokio::test]
async fn key_does_not_open_other_methods_to_a_request_from_another_site() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let headers = [("sec-fetch-site", "cross-site"), ("token", seed::API_KEY)];

        // The same-origin check for POST, PUT and DELETE runs before the route, and it
        // knows nothing about the key. A script sends no browser header and is let in.
        let from_a_page = send(&app, request("DELETE", "/api/v2/token/1", &headers, "")).await;
        let from_a_script = send(
            &app,
            request("DELETE", "/api/v2/token/1", &[("token", seed::API_KEY)], ""),
        )
        .await;

        assert_eq!(from_a_page.status, 401, "{}", from_a_page.body);
        assert_eq!(
            from_a_page.message(),
            json!("Not authorized for current Route: /api/v2/token/1")
        );
        assert_eq!(from_a_script.status, 204, "{}", from_a_script.body);
    })
    .await;
}
