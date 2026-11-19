//! The client address as the handlers of the real router see it.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, header};
use organizr_api::client::TrustedProxies;
use organizr_api::{AppState, router};
use organizr_db::Database;
use organizr_settings::Settings;
use tower::ServiceExt;

/// Asks the application for `/api/v2/ping` as a caller without the device cookie, over a
/// connection from `peer`, and returns the `Set-Cookie` values. Whether they end in
/// `Secure` shows whether the application believed the forwarding headers.
async fn device_cookies(trusted: &str, peer: &str, forwarded_proto: &str) -> Vec<String> {
    let directory = tempfile::tempdir().expect("a temporary folder can be created");
    let file = directory.path().join("organizr.db");
    let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));
    let database = Database::connect(&url).await.expect("the database opens");
    database.migrate().await.expect("the migrations apply");

    let trusted: TrustedProxies = trusted.parse().expect("the proxy list is valid");
    let state = AppState::new(
        database.clone(),
        Settings::defaults(),
        organizr_testkit::seed::secrets(),
    )
    .with_trusted_proxies(trusted);
    let app = router(state).expect("the route table is valid");

    let peer: SocketAddr = peer.parse().expect("the peer address is valid");
    let request = Request::builder()
        .uri("/api/v2/ping")
        .header(header::HOST, "organizr.test")
        .header("x-forwarded-for", "203.0.113.7")
        .header("x-forwarded-proto", forwarded_proto)
        .extension(ConnectInfo(peer))
        .body(Body::empty())
        .expect("the request is valid");
    let response = app.oneshot(request).await.expect("the router answers");

    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a cookie is text").to_owned())
        .collect();
    database.close().await.expect("the database closes");
    cookies
}

#[tokio::test]
async fn trusted_proxy_that_reports_https_makes_cookies_secure() {
    let cookies = device_cookies("172.18.0.0/16", "172.18.0.2:41000", "https").await;

    assert_eq!(cookies.len(), 2);
    for cookie in &cookies {
        assert!(cookie.ends_with("; Secure; HttpOnly"), "{cookie}");
    }
}

#[tokio::test]
async fn trusted_proxy_that_reports_http_leaves_cookies_plain() {
    let cookies = device_cookies("172.18.0.0/16", "172.18.0.2:41000", "http").await;

    assert_eq!(cookies.len(), 2);
    for cookie in &cookies {
        assert!(cookie.ends_with("; SameSite=Lax"), "{cookie}");
    }
}

#[tokio::test]
async fn caller_that_is_not_a_trusted_proxy_cannot_claim_https() {
    for trusted in ["", "172.18.0.0/16"] {
        let cookies = device_cookies(trusted, "203.0.113.9:41000", "https").await;

        assert_eq!(cookies.len(), 2);
        for cookie in &cookies {
            assert!(
                cookie.ends_with("; SameSite=Lax"),
                "trusted `{trusted}`: {cookie}"
            );
        }
    }
}
