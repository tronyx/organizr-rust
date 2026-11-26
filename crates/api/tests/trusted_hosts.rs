//! The allowed hosts and the trusted proxies as settings, which the variables of the
//! container win over (D-155).

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::net::SocketAddr;

use axum::Router;
use axum::extract::ConnectInfo;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

fn app_with(database: &Database, hosts: &str, proxies: &str) -> Router {
    let state = state_with(
        database,
        installed_settings(&[("loginAttempts", json!("1000000"))]),
    )
    .with_allowed_hosts(hosts.parse().expect("the hosts are valid"))
    .with_trusted_proxies(proxies.parse().expect("the proxies are valid"));
    router(state).expect("the route table is valid")
}

async fn lists(app: &Router) -> Value {
    let answer = call(app, As::Admin, "GET", "/trusted-hosts", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer.data()
}

async fn log_in_at(app: &Router, host: &str) -> Answer {
    let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
    let headers = [("host", host), ("content-type", "application/json")];
    send(
        app,
        request("POST", "/api/v2/login", &headers, &body.to_string()),
    )
    .await
}

#[tokio::test]
async fn settings_hold_the_lists_when_the_variables_are_empty() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with(database, "", "");
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        let shown = lists(&app).await;
        assert_eq!(shown["allowedHosts"]["value"], json!(""));
        assert_eq!(shown["allowedHosts"]["variable"], json!("ORGANIZR_ALLOWED_HOSTS"));
        assert_eq!(shown["allowedHosts"]["from_variable"], json!(false));
        assert_eq!(shown["trustedProxies"]["variable"], json!("ORGANIZR_TRUSTED_PROXIES"));
        let seen = call(&app, As::User(&co_admin), "GET", "/trusted-hosts", None).await;
        assert_eq!(seen.status, StatusCode::OK, "{}", seen.body);

        // Only an Admin changes them, and only to a list that the variable would take.
        let body = json!({"allowedHosts": "organizr.example.com"});
        let refused = call(&app, As::User(&co_admin), "PUT", "/config", Some(body)).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{}", refused.body);
        for (key, wrong) in [("allowedHosts", "https://organizr.example.com"), ("trustedProxies", "not-an-address")] {
            let answer = call(&app, As::Admin, "PUT", "/config", Some(json!({key: wrong}))).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{key}: {}", answer.body);
        }
        let body = json!({"allowedHosts": "Organizr.Example.com, 192.168.1.10:8080", "trustedProxies": "172.18.0.0/16"});
        let saved = call(&app, As::Admin, "PUT", "/config", Some(body)).await;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        let shown = lists(&app).await;
        assert_eq!(shown["allowedHosts"]["value"], json!("organizr.example.com,192.168.1.10:8080"));
        assert_eq!(shown["trustedProxies"]["value"], json!("172.18.0.0/16"));

        // The stored proxy is believed about the client, at once.
        let mut asked = request("GET", "/api/v2/auth/999", &[("x-forwarded-for", "203.0.113.5")], "");
        let proxy: SocketAddr = "172.18.0.2:50000".parse().expect("the address is valid");
        asked.extensions_mut().insert(ConnectInfo(proxy));
        let answer = send(&app, asked).await;
        assert_eq!(answer.data()["user_ip"], json!("203.0.113.5"), "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn variables_win_over_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with(database, "organizr.example.com", "10.0.0.1");

        let shown = lists(&app).await;
        assert_eq!(
            shown["allowedHosts"]["value"],
            json!("organizr.example.com")
        );
        assert_eq!(shown["allowedHosts"]["from_variable"], json!(true));
        assert_eq!(shown["trustedProxies"]["value"], json!("10.0.0.1/32"));
        assert_eq!(shown["trustedProxies"]["from_variable"], json!(true));
        for key in ["allowedHosts", "trustedProxies"] {
            let answer = call(
                &app,
                As::Admin,
                "PUT",
                "/config",
                Some(json!({key: "192.168.1.10"})),
            )
            .await;
            assert_eq!(
                answer.status,
                StatusCode::CONFLICT,
                "{key}: {}",
                answer.body
            );
            assert!(
                answer.body.contains("in the environment of the container"),
                "{}",
                answer.body
            );
        }

        // An Admin login stores no host while the variable names some.
        assert_eq!(
            log_in_at(&app, "elsewhere.example.com").await.status,
            StatusCode::OK
        );
        assert_eq!(
            lists(&app).await["allowedHosts"]["value"],
            json!("organizr.example.com")
        );
    })
    .await;
}

#[tokio::test]
async fn admin_login_stores_its_host_once_when_no_host_is_named() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with(database, "", "");

        // As after an import: the first Admin login names the host of Organizr, so a
        // service address like /sonarr works there at once (D-155).
        assert_eq!(
            log_in_at(&app, "organizr.example.com:8080").await.status,
            StatusCode::OK
        );
        assert_eq!(
            lists(&app).await["allowedHosts"]["value"],
            json!("organizr.example.com:8080")
        );
        assert_eq!(
            log_in_at(&app, "other.example.com").await.status,
            StatusCode::OK
        );
        assert_eq!(
            lists(&app).await["allowedHosts"]["value"],
            json!("organizr.example.com:8080")
        );
    })
    .await;
}
