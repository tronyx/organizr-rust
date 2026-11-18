//! The device cookie as a caller sees it: who gets one, and what it looks like.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::from_fn;
use axum::routing::get;
use organizr_api::device_cookie::{DEVICE_COOKIE, ensure_device_cookie};
use tower::ServiceExt;

fn app() -> Router {
    Router::new()
        .route("/", get(|| async { "ok" }))
        .layer(from_fn(ensure_device_cookie))
}

/// Sends a request to `path` and returns the status and the `Set-Cookie` values.
async fn cookies_set(path: &str, request_headers: &[(&str, &str)]) -> (StatusCode, Vec<String>) {
    let mut request = Request::builder().uri(path);
    for (name, value) in request_headers {
        request = request.header(*name, *value);
    }
    let response = app()
        .oneshot(request.body(Body::empty()).expect("the request is valid"))
        .await
        .expect("the router answers");
    let cookies = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .map(|value| value.to_str().expect("a cookie is text").to_owned())
        .collect();
    (response.status(), cookies)
}

/// The value of the device cookie in a `Set-Cookie` header.
fn id_of(cookie: &str) -> &str {
    cookie
        .strip_prefix(DEVICE_COOKIE)
        .and_then(|rest| rest.strip_prefix('='))
        .and_then(|rest| rest.split(';').next())
        .expect("the header sets the device cookie")
}

#[tokio::test]
async fn caller_without_the_cookie_gets_it_twice_with_one_id() {
    let (status, cookies) = cookies_set("/", &[("host", "app.example.com")]).await;

    assert_eq!(status, StatusCode::OK);
    let [wide, host] = cookies.as_slice() else {
        panic!("expected two cookies, got {cookies:?}");
    };
    assert!(wide.contains("; domain=.example.com;"), "{wide}");
    assert!(host.contains("; domain=app.example.com;"), "{host}");
    assert_eq!(id_of(wide), id_of(host));
}

#[tokio::test]
async fn id_is_a_random_version_4_uuid() {
    let (_, first) = cookies_set("/", &[("host", "organizr.test")]).await;
    let (_, second) = cookies_set("/", &[("host", "organizr.test")]).await;

    let first = id_of(first.first().expect("a cookie is set"));
    let second = id_of(second.first().expect("a cookie is set"));

    assert_ne!(first, second, "each caller gets an id of its own");
    for id in [first, second] {
        let groups: Vec<&str> = id.split('-').collect();
        let lengths: Vec<usize> = groups.iter().map(|group| group.len()).collect();
        assert_eq!(lengths, [8, 4, 4, 4, 12], "`{id}` has the form of a UUID");
        assert!(
            id.chars()
                .all(|character| character == '-' || matches!(character, '0'..='9' | 'a'..='f')),
            "`{id}` is lower-case hexadecimal"
        );
        assert!(groups[2].starts_with('4'), "`{id}` is version 4");
    }
}

#[tokio::test]
async fn caller_with_the_cookie_gets_no_new_one() {
    let (status, cookies) = cookies_set(
        "/",
        &[
            ("host", "organizr.test"),
            (
                "cookie",
                "organizr_user_uuid=2adf6c3d-5376-47cd-8bee-9952a2739385",
            ),
        ],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(cookies, Vec::<String>::new());
}

#[tokio::test]
async fn cookie_is_also_set_on_an_answer_that_is_not_a_success() {
    let (status, cookies) = cookies_set("/missing", &[("host", "organizr.test")]).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(cookies.len(), 2);
}

#[tokio::test]
async fn cookie_over_https_is_secure() {
    let (_, cookies) = cookies_set(
        "/",
        &[("host", "organizr.test"), ("x-forwarded-proto", "https")],
    )
    .await;

    assert_eq!(cookies.len(), 2);
    for cookie in &cookies {
        assert!(
            cookie.ends_with("; SameSite=Lax; Secure; HttpOnly"),
            "{cookie}"
        );
    }
}
