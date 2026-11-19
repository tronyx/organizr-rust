//! The same-origin check must leave alone what it is not meant to stop.
//!
//! The application has no route yet that changes something, so the refusal itself is
//! tested in the router, around a test route. These tests go through the real route table.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use organizr_testkit::app;
use organizr_testkit::engines::on_each_engine;
use tower::ServiceExt;

/// Sends a request with the given browser headers and returns the status and the body.
async fn send(
    app: &Router,
    method: Method,
    path: &str,
    browser_headers: &[(&str, &str)],
) -> (StatusCode, String) {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::HOST, "organizr.example.com");
    for (name, value) in browser_headers {
        request = request.header(*name, *value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).expect("the request is valid"))
        .await
        .expect("the router answers");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes();
    (
        status,
        String::from_utf8(body.to_vec()).expect("the body is UTF-8"),
    )
}

#[tokio::test]
async fn request_from_the_application_itself_is_let_through() {
    on_each_engine(async |database| {
        let app = app(database);
        let from_here: [&[(&str, &str)]; 4] = [
            // A current browser on a page of the application.
            &[("sec-fetch-site", "same-origin")],
            // A request the user started directly, such as from a bookmark.
            &[("sec-fetch-site", "none")],
            // An older browser on a page of the application.
            &[("origin", "https://organizr.example.com")],
            // A script or an API client: no browser headers at all.
            &[],
        ];

        for headers in from_here {
            let (status, body) = send(&app, Method::POST, "/api/v2/rox", headers).await;

            // The check lets the request through. No route takes a POST here, so the
            // answer is the ordinary "not found".
            assert_eq!(status, StatusCode::NOT_FOUND, "{headers:?}");
            assert!(
                body.contains("Endpoint Not Found or Defined"),
                "{headers:?}: {body}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn reading_is_never_refused() {
    on_each_engine(async |database| {
        let app = app(database);

        for method in [Method::GET, Method::HEAD] {
            let (status, _) = send(
                &app,
                method.clone(),
                "/api/v2/ping",
                &[("sec-fetch-site", "cross-site")],
            )
            .await;

            assert_eq!(status, StatusCode::OK, "{method}");
        }
    })
    .await;
}

#[tokio::test]
async fn path_that_is_no_route_is_not_found_whoever_asks() {
    on_each_engine(async |database| {
        let app = app(database);

        let (status, body) = send(
            &app,
            Method::POST,
            "/api/v2/nothing/here",
            &[("sec-fetch-site", "cross-site")],
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body.contains("Endpoint Not Found or Defined"), "{body}");
    })
    .await;
}
