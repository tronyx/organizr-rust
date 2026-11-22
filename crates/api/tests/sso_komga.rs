//! Single sign-on to Komga: a login gets a session of Komga for the e-mail address of the
//! user, and a logout ends it, as in PHP.

#![allow(
    clippy::expect_used,
    reason = "a test reads the answers it knows, and its helpers may panic"
)]

use axum::http::StatusCode;
use organizr_db::Database;
use organizr_domain::{NewUser, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::{self, KOMGA_ROBIN};
use organizr_testkit::{installed_app_with, seed};
use serde_json::json;

/// Stores robin, with the e-mail address and the password that Komga knows, in the group
/// of the given level.
async fn store_robin(database: &Database, level: u16) {
    let level = organizr_domain::GroupLevel::new(level).expect("a level");
    let group = database
        .groups()
        .find_by_level(level)
        .await
        .expect("the lookup works")
        .expect("the install has the group");
    database
        .users()
        .insert(NewUser {
            username: "robin".to_owned(),
            email: Some(KOMGA_ROBIN.0.to_owned()),
            password_hash: organizr_auth::password::hash(KOMGA_ROBIN.1).expect("hashed"),
            group: group.id,
            image: None,
            created_at: Timestamp::from_unix_seconds(1_800_000_000),
        })
        .await
        .expect("the user can be stored");
}

fn cookie<'a>(answer: &'a Answer, name: &str) -> Option<&'a str> {
    answer
        .cookies
        .iter()
        .map(String::as_str)
        .find(|cookie| cookie.starts_with(&format!("{name}=")))
}

#[tokio::test]
async fn login_gets_a_session_of_komga_and_a_logout_ends_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_robin(database, 4).await;
        let komga = services::komga().await;
        let app = installed_app_with(
            database,
            &[("ssoKomga", json!(true)), ("komgaURL", json!(komga.url()))],
        );

        let body = json!({"username": "robin", "password": KOMGA_ROBIN.1});
        let answer = call(&app, As::Guest, "POST", "/login", Some(body)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let komga_cookie = cookie(&answer, "komga_token").expect("the Komga cookie is set");
        assert!(
            komga_cookie.starts_with("komga_token=komga-session-of-robin;"),
            "{komga_cookie}"
        );

        // A logout ends the session at Komga and drops the cookie.
        let session = answer
            .cookies
            .iter()
            .find(|cookie| cookie.starts_with("organizr_token_"))
            .and_then(|cookie| cookie.split(';').next())
            .expect("a session cookie")
            .to_owned();
        let sent = format!("{session}; komga_token=komga-session-of-robin");
        let out = call(&app, As::User(&sent), "GET", "/logout", None).await;
        assert_eq!(out.status, StatusCode::OK, "{}", out.body);
        assert!(
            cookie(&out, "komga_token").is_some(),
            "the cookie is dropped"
        );
        let last = komga.seen().pop().expect("Komga was asked");
        assert_eq!(last.path, "/api/logout");
        assert_eq!(last.header("x-auth-token"), Some("komga-session-of-robin"));
    })
    .await;
}

#[tokio::test]
async fn only_users_of_the_level_of_the_setting_get_a_session_of_komga() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        store_robin(database, 4).await;
        let komga = services::komga().await;
        let app = installed_app_with(
            database,
            &[
                ("ssoKomga", json!(true)),
                ("komgaURL", json!(komga.url())),
                ("ssoKomgaAuth", json!("1")),
            ],
        );

        let body = json!({"username": "robin", "password": KOMGA_ROBIN.1});
        let answer = call(&app, As::Guest, "POST", "/login", Some(body)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert!(cookie(&answer, "komga_token").is_none());
        assert!(komga.seen().is_empty(), "Komga is not asked");
    })
    .await;
}
