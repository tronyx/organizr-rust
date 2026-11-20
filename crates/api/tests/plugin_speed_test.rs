//! The routes of the speed test plugin.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::json;
use tower::ServiceExt;

const MEBIBYTE: usize = 1024 * 1024;

/// Sends a request as the Admin and returns the status, one header and the size of
/// the body.
async fn raw(
    app: &Router,
    method: &str,
    path: &str,
    body: Vec<u8>,
) -> (StatusCode, Option<String>, usize) {
    let request = Request::builder()
        .method(method)
        .uri(format!("/api/v2{path}"))
        .header("token", seed::API_KEY)
        .body(Body::from(body))
        .expect("the request is valid");
    let response = app.clone().oneshot(request).await.expect("the app answers");
    let status = response.status();
    let length = response
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let size = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes()
        .len();
    (status, length, size)
}

#[tokio::test]
async fn download_sends_as_many_mebibytes_as_the_query_asks_for() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("SPEEDTEST-enabled", json!(true))]);

        let (status, length, size) = raw(
            &app,
            "GET",
            "/plugins/speedtest/garbage?ckSize=3",
            Vec::new(),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(size, 3 * MEBIBYTE);
        assert_eq!(length, Some((3 * MEBIBYTE).to_string()));

        let (_, _, size) = raw(&app, "GET", "/plugins/speedtest/garbage", Vec::new()).await;
        assert_eq!(size, 4 * MEBIBYTE);
    })
    .await;
}

#[tokio::test]
async fn upload_counts_the_bytes_and_ping_and_address_answer() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[("SPEEDTEST-enabled", json!(true))]);

        // Larger than the limit of an ordinary request body.
        let request = Request::builder()
            .method("POST")
            .uri("/api/v2/plugins/speedtest/empty")
            .header("token", seed::API_KEY)
            .body(Body::from(vec![7_u8; 5 * MEBIBYTE]))
            .expect("the request is valid");
        let response = app.clone().oneshot(request).await.expect("the app answers");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response
            .into_body()
            .collect()
            .await
            .expect("a body")
            .to_bytes();
        let answer: serde_json::Value = serde_json::from_slice(&body).expect("JSON");
        assert_eq!(
            answer["response"]["data"],
            json!({"received": 5 * MEBIBYTE})
        );

        let (status, _, size) = raw(&app, "GET", "/plugins/speedtest/empty", Vec::new()).await;
        assert_eq!((status, size), (StatusCode::OK, 0));

        let address = call(&app, As::Admin, "GET", "/plugins/speedtest/ip", None).await;
        assert_eq!(address.status, StatusCode::OK, "{}", address.body);
        assert!(
            address.data()["ip"]
                .as_str()
                .is_some_and(|ip| !ip.is_empty())
        );
    })
    .await;
}

#[tokio::test]
async fn routes_need_the_plugin_to_be_on_and_the_level_of_its_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let paths = [
            "/plugins/speedtest/garbage",
            "/plugins/speedtest/empty",
            "/plugins/speedtest/ip",
        ];

        let off = installed_app_with(database, &[]);
        for path in paths {
            let refused = call(&off, As::Admin, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::CONFLICT, "{path}");
        }
        let refused = call(&off, As::Admin, "POST", "/plugins/speedtest/empty", None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);

        // The level of the plugin is 1: Admins and Co-Admins.
        let on = installed_app_with(database, &[("SPEEDTEST-enabled", json!(true))]);
        let user = seed::logged_in(database, &on, "zed", 4).await;
        for path in paths {
            for who in [As::Guest, As::User(&user)] {
                let refused = call(&on, who, "GET", path, None).await;
                assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
            }
        }
        let open = installed_app_with(
            database,
            &[
                ("SPEEDTEST-enabled", json!(true)),
                ("SPEEDTEST-Auth-include", json!("4")),
            ],
        );
        let passed = call(&open, As::User(&user), "GET", "/plugins/speedtest/ip", None).await;
        assert_eq!(passed.status, StatusCode::OK);
    })
    .await;
}
