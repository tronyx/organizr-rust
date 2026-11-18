//! The limit on failed logins, through the real route table.
//!
//! `crates/testkit/tests/parity.rs` replays one visit of a browser that PHP locked out.
//! These tests look at what PHP does not have: the count is on the server, by address
//! and by name, and nothing the client sends changes it.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{installed_app, installed_app_with, seed};
use serde_json::{Value, json};

const HOME: &str = "198.51.100.7";
const ELSEWHERE: &str = "203.0.113.50";

/// Sends a login from the given address with the given body and cookies.
async fn login_with(app: &Router, address: &str, body: &Value, cookies: &str) -> Answer {
    let mut headers = vec![("content-type", "application/json")];
    if !cookies.is_empty() {
        headers.push(("cookie", cookies));
    }
    let mut request = request("POST", "/api/v2/login", &headers, &body.to_string());
    let peer: SocketAddr = format!("{address}:50000")
        .parse()
        .expect("the address is valid");
    request.extensions_mut().insert(ConnectInfo(peer));
    send(app, request).await
}

async fn login(app: &Router, address: &str, name: &str, password: &str) -> Answer {
    let body = json!({"username": name, "password": password});
    login_with(app, address, &body, "").await
}

/// Fails `count` logins for a name from an address and returns the last answer.
async fn fail(app: &Router, address: &str, name: &str, count: usize) -> Answer {
    let mut last = None;
    for _ in 0..count {
        last = Some(login(app, address, name, "not-it").await);
    }
    last.expect("at least one login is sent")
}

fn is_lock_out(answer: &Answer) -> bool {
    answer.status == 403 && answer.message() == json!("User is locked out")
}

#[tokio::test]
async fn fifth_failed_login_locks_out_and_tells_how_long() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let fourth = fail(&app, HOME, seed::ADMIN_NAME, 4).await;
        let fifth = fail(&app, HOME, seed::ADMIN_NAME, 1).await;
        let correct = login(&app, HOME, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;

        assert_eq!(fourth.status, 401, "{}", fourth.body);
        assert_eq!(fourth.headers.get("retry-after"), None);
        assert!(is_lock_out(&fifth), "{}", fifth.body);
        assert!(is_lock_out(&correct), "{}", correct.body);
        assert_eq!(correct.cookies_named("lockout"), Vec::<&str>::new());
        for answer in [&fifth, &correct] {
            let seconds: u64 = answer
                .headers
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok())
                .expect("the answer says how long to wait");
            // The default lock-out is 60 000 milliseconds.
            assert!((55..=60).contains(&seconds), "{seconds}");
        }
    })
    .await;
}

#[tokio::test]
async fn what_the_client_says_about_its_own_count_changes_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let correct = |extra: Value| {
            let mut body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
            body.as_object_mut()
                .expect("the body is an object")
                .extend(extra.as_object().expect("the fields are an object").clone());
            body
        };

        // PHP refuses these two: it believes the count and the cookie of the client.
        let high_count = login_with(&app, HOME, &correct(json!({"loginAttempts": 99})), "").await;
        let with_cookie = login_with(&app, HOME, &correct(json!({})), "lockout=60000").await;
        assert_eq!(high_count.status, 200, "{}", high_count.body);
        assert_eq!(with_cookie.status, 200, "{}", with_cookie.body);

        // And a client that always says "first try" is locked out all the same.
        let wrong = json!({"username": seed::ADMIN_NAME, "password": "not-it", "loginAttempts": 1});
        let mut last = None;
        for _ in 0..5 {
            last = Some(login_with(&app, ELSEWHERE, &wrong, "").await);
        }
        assert!(is_lock_out(&last.expect("five logins were sent")));
    })
    .await;
}

#[tokio::test]
async fn lock_follows_the_address_and_the_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        // One address tries five names.
        for name in ["anna", "ben", "carl", "dora"] {
            let answer = fail(&app, HOME, name, 1).await;
            assert_eq!(answer.status, 401, "{name}");
        }
        assert!(is_lock_out(&fail(&app, HOME, "emil", 1).await));

        // The address is locked for every name, also with the correct password.
        let from_home = login(&app, HOME, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert!(is_lock_out(&from_home), "{}", from_home.body);
        // None of the names is locked: each failed once. Another address can log in.
        let from_elsewhere = login(&app, ELSEWHERE, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert_eq!(from_elsewhere.status, 200, "{}", from_elsewhere.body);

        // Five addresses try one name.
        for address in ["192.0.2.1", "192.0.2.2", "192.0.2.3", "192.0.2.4"] {
            let answer = fail(&app, address, "Admin", 1).await;
            assert_eq!(answer.status, 401, "{address}");
        }
        assert!(is_lock_out(&fail(&app, "192.0.2.5", "ADMIN", 1).await));

        // The name is locked from every address, also with the correct password.
        let for_admin = login(&app, ELSEWHERE, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        assert!(is_lock_out(&for_admin), "{}", for_admin.body);
        // The addresses are not locked: each failed once.
        let other_name = fail(&app, "192.0.2.1", "nobody", 1).await;
        assert_eq!(other_name.status, 401, "{}", other_name.body);
    })
    .await;
}

#[tokio::test]
async fn successful_login_sets_the_count_back() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        fail(&app, HOME, seed::ADMIN_NAME, 4).await;
        let correct = login(&app, HOME, seed::ADMIN_NAME, seed::ADMIN_PASSWORD).await;
        let after = fail(&app, HOME, seed::ADMIN_NAME, 4).await;

        assert_eq!(correct.status, 200, "{}", correct.body);
        assert_eq!(
            after.status, 401,
            "four new failures do not lock: {}",
            after.body
        );
        assert!(is_lock_out(&fail(&app, HOME, seed::ADMIN_NAME, 1).await));
    })
    .await;
}

#[tokio::test]
async fn limit_and_lock_out_time_come_from_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("loginAttempts", json!("2")),
                ("loginLockout", json!("120000")),
            ],
        );

        let first = fail(&app, HOME, seed::ADMIN_NAME, 1).await;
        let second = fail(&app, HOME, seed::ADMIN_NAME, 1).await;

        assert_eq!(first.status, 401, "{}", first.body);
        assert!(is_lock_out(&second), "{}", second.body);
        assert_eq!(
            second
                .headers
                .get("retry-after")
                .and_then(|value| value.to_str().ok()),
            Some("120")
        );
    })
    .await;
}

#[tokio::test]
async fn lock_out_does_not_stop_other_routes_or_a_session_that_exists() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let session = login(&app, HOME, seed::ADMIN_NAME, seed::ADMIN_PASSWORD)
            .await
            .cookies
            .iter()
            .find(|cookie| cookie.starts_with("organizr_token_"))
            .and_then(|cookie| cookie.split(';').next())
            .expect("the login sets the session cookie")
            .to_owned();
        assert!(is_lock_out(&fail(&app, HOME, seed::ADMIN_NAME, 5).await));

        let me = send(
            &app,
            request("GET", "/api/v2/token/me", &[("cookie", &session)], ""),
        )
        .await;

        assert_eq!(me.status, 200);
        assert_eq!(me.data()["username"], json!("admin"));
    })
    .await;
}
