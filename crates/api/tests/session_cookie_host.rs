//! The switch `ORGANIZR_SESSION_COOKIE_HOST_ONLY` of the operator: the session cookie
//! for the host of Organizr alone, so that the other hosts of the domain never receive
//! it. Without the switch the cookie is set as in PHP, for the wider domain too.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::json;

const SESSION_COOKIE: &str = "organizr_token_5d0c6f0e-2b7a-4a36-8e55-9c1f4d3a7b21";

fn app(database: &Database, host_only: bool) -> Router {
    let state =
        state_with(database, installed_settings(&[])).with_session_cookie_host_only(host_only);
    organizr_api::router(state).expect("the route table is valid")
}

async fn log_in(app: &Router) -> Answer {
    let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
    let answer = call(app, As::Guest, "POST", "/login", Some(body)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
}

/// The `Set-Cookie` values of the session cookie at `/`, with a value or without.
fn at_root(answer: &Answer, with_value: bool) -> Vec<String> {
    answer
        .cookies_named(SESSION_COOKIE)
        .into_iter()
        .filter(|cookie| cookie.contains("; path=/;"))
        .filter(|cookie| cookie.starts_with(&format!("{SESSION_COOKIE}=;")) != with_value)
        .map(str::to_owned)
        .collect()
}

#[tokio::test]
async fn session_cookie_is_for_the_wider_domain_without_the_switch() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let answer = log_in(&app(database, false)).await;

        let set = at_root(&answer, true);
        assert_eq!(set.len(), 2, "{set:?}");
        assert!(set[0].contains("; domain=.example.com;"), "{set:?}");
        assert!(set[1].contains("; domain=organizr.example.com;"), "{set:?}");
    })
    .await;
}

#[tokio::test]
async fn switch_makes_the_session_cookie_one_for_the_host_alone() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app(database, true);
        let answer = log_in(&app).await;

        // One cookie without a domain, which the browser sends to this host alone.
        let set = at_root(&answer, true);
        assert_eq!(set.len(), 1, "{set:?}");
        assert!(!set[0].contains("domain="), "{set:?}");
        // The cookies of the domain from before the switch are dropped first.
        let dropped = at_root(&answer, false);
        assert!(
            dropped
                .iter()
                .any(|cookie| cookie.contains("; domain=.example.com;")),
            "{dropped:?}"
        );
        let position = |cookie: &str| {
            answer
                .cookies
                .iter()
                .position(|value| value == cookie)
                .expect("the cookie is in the answer")
        };
        assert!(
            dropped
                .iter()
                .all(|cookie| position(cookie) < position(&set[0]))
        );

        // The logout drops the cookie of the host too.
        let session = set[0].split(';').next().expect("a value").to_owned();
        let out = call(&app, As::User(&session), "GET", "/logout", None).await;
        assert!(
            at_root(&out, false)
                .iter()
                .any(|cookie| !cookie.contains("domain=")),
            "{:?}",
            out.cookies
        );
    })
    .await;
}
