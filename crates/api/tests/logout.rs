//! `GET /logout` through the real route table: what the recorded PHP answers cannot show.
//!
//! The answers themselves are compared with PHP in `crates/testkit/tests/parity.rs`. These
//! tests look at what a logout does to the stored sessions, and at who may log a user out.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use organizr_db::Database;
use organizr_domain::{SessionId, Timestamp};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::{installed_app, seed};
use serde_json::json;
use tower::ServiceExt;

/// The name of the session cookie of the seeded install.
const SESSION_COOKIE: &str = "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

/// The parts of an answer the tests look at.
struct Answer {
    status: StatusCode,
    cookies: Vec<String>,
    body: String,
}

async fn send(app: &Router, request: Request<Body>) -> Answer {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("the router answers");
    let status = response.status();
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a cookie is text").to_owned())
        .collect();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes();
    Answer {
        status,
        cookies,
        body: String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
    }
}

/// Logs the admin in and returns the session cookie as a browser sends it back.
async fn log_in(app: &Router) -> String {
    let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
    let request = Request::builder()
        .method("POST")
        .uri("/api/v2/login")
        .header(header::HOST, "organizr.example.com")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("the request is valid");
    let answer = send(app, request).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with(SESSION_COOKIE))
        .and_then(|cookie| cookie.split(';').next())
        .expect("the login sets the session cookie")
        .to_owned()
}

async fn log_out(app: &Router, cookies: &str, headers: &[(&str, &str)]) -> Answer {
    let mut request = Request::builder()
        .method("GET")
        .uri("/api/v2/logout")
        .header(header::HOST, "organizr.example.com");
    if !cookies.is_empty() {
        request = request.header(header::COOKIE, cookies);
    }
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    send(
        app,
        request.body(Body::empty()).expect("the request is valid"),
    )
    .await
}

/// The ids of the stored sessions of the admin account.
async fn session_ids(database: &Database) -> Vec<SessionId> {
    let admin = database
        .users()
        .find_by_username(seed::ADMIN_NAME)
        .await
        .expect("the lookup works")
        .expect("the seed has the admin account");
    database
        .sessions()
        .list_valid_of_user(admin.id, Timestamp::from_unix_seconds(0))
        .await
        .expect("the sessions can be read")
        .into_iter()
        .map(|session| session.id)
        .collect()
}

#[tokio::test]
async fn logout_deletes_the_session_of_the_cookie_and_no_other() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let this_browser = log_in(&app).await;
        let _other_browser = log_in(&app).await;
        let before = session_ids(database).await;
        let [first, second] = before.as_slice() else {
            unreachable!("two logins store two sessions, not {}", before.len());
        };

        let answer = log_out(&app, &this_browser, &[]).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(session_ids(database).await, [*second], "{first} is gone");

        // The cookie of the deleted session does nothing any more.
        let again = log_out(&app, &this_browser, &[]).await;
        assert_eq!(again.status, StatusCode::OK, "{}", again.body);
        assert_eq!(session_ids(database).await, [*second]);
    })
    .await;
}

#[tokio::test]
async fn logout_leaves_the_browser_with_a_new_device_id() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let old_device = "organizr_user_uuid=0e3b1d7c-5f0a-4c59-9f3c-2a6d1b8e4f70";

        for cookies in ["", old_device] {
            let answer = log_out(&app, cookies, &[]).await;

            // A browser applies the cookies in order, so the last one stays.
            let last = answer
                .cookies
                .iter()
                .rfind(|cookie| cookie.starts_with("organizr_user_uuid="))
                .and_then(|cookie| cookie.split(';').next())
                .expect("the logout sets the device cookie");
            assert_ne!(last, "organizr_user_uuid=", "sent: {cookies:?}");
            assert_ne!(last, old_device, "sent: {cookies:?}");
        }
    })
    .await;
}

#[tokio::test]
async fn cookie_that_names_no_session_changes_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        log_in(&app).await;
        let before = session_ids(database).await;
        let no_sessions = [
            // A token that no login made.
            format!("{SESSION_COOKIE}={}", "ab".repeat(32)),
            // A cookie of the PHP application.
            format!("{SESSION_COOKIE}=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.e30.c2ln"),
            format!("{SESSION_COOKIE}="),
            // The cookie of another install.
            format!("organizr_token_temp={}", "ab".repeat(32)),
        ];

        for cookies in no_sessions {
            let answer = log_out(&app, &cookies, &[]).await;

            assert_eq!(answer.status, StatusCode::OK, "{cookies}");
            assert_eq!(session_ids(database).await, before, "{cookies}");
        }
    })
    .await;
}

#[tokio::test]
async fn another_site_cannot_log_a_user_out() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let session = log_in(&app).await;
        let before = session_ids(database).await;
        let from_elsewhere: [&[(&str, &str)]; 3] = [
            // A link or an image on another site.
            &[("sec-fetch-site", "cross-site")],
            &[("sec-fetch-site", "same-site")],
            &[("origin", "https://evil.example.net")],
        ];

        for headers in from_elsewhere {
            let answer = log_out(&app, &session, headers).await;

            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{headers:?}");
            assert!(
                answer
                    .body
                    .contains("Not authorized for current Route: /api/v2/logout"),
                "{headers:?}: {}",
                answer.body
            );
            assert!(
                !answer
                    .cookies
                    .iter()
                    .any(|cookie| cookie.starts_with(SESSION_COOKIE)),
                "{headers:?}: the session cookie is not dropped"
            );
        }
        assert_eq!(session_ids(database).await, before);

        // The application itself, a bookmark, and a script can.
        let from_here: [&[(&str, &str)]; 3] = [
            &[("sec-fetch-site", "same-origin")],
            &[("sec-fetch-site", "none")],
            &[],
        ];
        for headers in from_here {
            let answer = log_out(&app, "", headers).await;
            assert_eq!(answer.status, StatusCode::OK, "{headers:?}");
        }
    })
    .await;
}
