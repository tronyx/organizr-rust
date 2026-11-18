//! The whole route table of the PHP application, expanded and served by a real Axum router.
//!
//! This proves three things the design depends on:
//!
//! - every pattern of Organizr 2.1.5000 expands, and Axum accepts all the routes together
//! - requests reach the route that Slim would choose, including the unusual shapes
//! - the cross-site request check of tower-http works as a layer on an Axum router
//!
//! The expectations for Slim are checked against recorded answers of the PHP application.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::body::Body;
use axum::extract::RawPathParams;
use axum::http::{Method, Request, StatusCode};
use axum::routing::{MethodRouter, any, delete, get, post, put};
use http_body_util::BodyExt;
use organizr_api::slim_route::{RoutePatternError, expand};
use tower::ServiceExt;
use tower_http::csrf::CsrfLayer;

const ROUTES: &str = include_str!("fixtures/organizr_routes.txt");

/// What the PHP application answered to the routing requests. `parity/recorder/record.py`
/// records it from the reference container.
const RECORDED_ROUTING: &str = include_str!("../../../parity/cases/routing.json");

/// The pattern that matches every path. Slim registers it last, as the "not found" answer.
const CATCH_ALL: &str = "{route:.*}";

/// The answer of the router's fallback.
const FALLBACK: &str = "fallback";

/// The fixture's routes as `(method, pattern)`.
fn organizr_routes() -> Vec<(String, String)> {
    ROUTES
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (method, pattern) = line.split_once(' ').expect("a line is `METHOD 'pattern'`");
            let pattern = pattern
                .strip_prefix('\'')
                .and_then(|pattern| pattern.strip_suffix('\''))
                .expect("the pattern is in single quotes");
            (method.to_owned(), pattern.to_owned())
        })
        .collect()
}

/// A router with every Organizr route under `/api/v2`.
///
/// Each route answers with its pattern and the values of its parameters, so a test can
/// see which route a request reached.
fn organizr_router() -> Router {
    let mut router = Router::new();
    for (method, pattern) in organizr_routes() {
        // The empty pattern can never match in the PHP application: Slim only applies its
        // base path when the request path holds `api/v2/`, and then the rest is not empty.
        // The recorded answer to `GET /api/v2` is "not found".
        if pattern == CATCH_ALL || pattern.is_empty() {
            continue;
        }
        for route in expand(&pattern).expect("the pattern expands") {
            let label = pattern.clone();
            let handler = move |params: RawPathParams| async move {
                let values: Vec<&str> = params.iter().map(|(_, value)| value).collect();
                format!("{label} <{}>", values.join("> <"))
            };
            let method_router: MethodRouter = match method.as_str() {
                "ANY" => any(handler),
                "GET" | "REDIRECT" => get(handler),
                "POST" => post(handler),
                "PUT" => put(handler),
                "DELETE" => delete(handler),
                other => panic!("the fixture has an unknown method `{other}`"),
            };
            router = router.route(&format!("/api/v2{}", route.path()), method_router);
        }
    }
    router
        .fallback(|| async { FALLBACK })
        .method_not_allowed_fallback(|| async { FALLBACK })
}

/// Sends one request and returns the status and the body.
async fn send(router: &Router, request: Request<Body>) -> (StatusCode, String) {
    let response = router
        .clone()
        .oneshot(request)
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

/// Sends a request without a body and returns the answer's body.
async fn reach(router: &Router, method: Method, path: &str) -> String {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("the request is valid");
    send(router, request).await.1
}

/// Requests and the route each one must reach: method, path, expected answer.
const ROUTING_CASES: &[(Method, &str, &str)] = &[
    // The API root: only the form with the slash is a route.
    (Method::GET, "/api/v2", FALLBACK),
    (Method::GET, "/api/v2/", "/ <>"),
    // An optional trailing slash.
    (Method::GET, "/api/v2/status", "/status[/] <>"),
    (Method::GET, "/api/v2/status/", "/status[/] <>"),
    (Method::GET, "/api/v2/launch/", "/launch[/] <>"),
    // Forward-auth: every level of the nested optional parts, on any method.
    (
        Method::GET,
        "/api/v2/auth",
        "/auth[/[{group}[/{type}[/{ips}]]]] <>",
    ),
    (
        Method::POST,
        "/api/v2/auth/",
        "/auth[/[{group}[/{type}[/{ips}]]]] <>",
    ),
    (
        Method::GET,
        "/api/v2/auth/1",
        "/auth[/[{group}[/{type}[/{ips}]]]] <1>",
    ),
    (
        Method::GET,
        "/api/v2/auth/1/allow/10.0.0.1,10.0.0.2",
        "/auth[/[{group}[/{type}[/{ips}]]]] <1> <allow> <10.0.0.1,10.0.0.2>",
    ),
    (
        Method::PUT,
        "/api/v2/organizr-auth/2/deny",
        "/organizr-auth[/[{group}[/{type}[/{ips}]]]] <2> <deny>",
    ),
    // Forward-auth: the group joined to the route name.
    (Method::GET, "/api/v2/auth-", "/auth-[{group}[/]] <>"),
    (Method::GET, "/api/v2/auth-5", "/auth-[{group}[/]] <5>"),
    (
        Method::GET,
        "/api/v2/auth-Plex/",
        "/auth-[{group}[/]] <Plex>",
    ),
    // The proxy routes: the rest of the path in one parameter, also when it is empty.
    (
        Method::GET,
        "/api/v2/socks/sonarr/api/v3/queue",
        "/socks/{app}/{route:.*} <sonarr> <api/v3/queue>",
    ),
    (
        Method::DELETE,
        "/api/v2/socks/sonarr/",
        "/socks/{app}/{route:.*} <sonarr>",
    ),
    (
        Method::POST,
        "/api/v2/multiple/socks/radarr/2/api/v3/command",
        "/multiple/socks/{app}/{instance}/{route:.*} <radarr> <2> <api/v3/command>",
    ),
    (Method::GET, "/api/v2/socks/sonarr", FALLBACK),
    // Routes that differ only by a trailing slash.
    (
        Method::GET,
        "/api/v2/homepage/donate",
        "/homepage/donate <>",
    ),
    (
        Method::GET,
        "/api/v2/homepage/donate/",
        "/homepage/donate/[{amount}] <>",
    ),
    (
        Method::GET,
        "/api/v2/homepage/donate/5",
        "/homepage/donate/[{amount}] <5>",
    ),
    (
        Method::GET,
        "/api/v2/homepage/donate/success",
        "/homepage/donate/success <>",
    ),
    (
        Method::POST,
        "/api/v2/homepage/jackett/download/",
        "/homepage/jackett/download/ <>",
    ),
    (Method::POST, "/api/v2/homepage/jackett/download", FALLBACK),
    (
        Method::GET,
        "/api/v2/homepage/jackett/download",
        "/homepage/jackett/{query} <download>",
    ),
    // Optional parameters.
    (
        Method::GET,
        "/api/v2/log",
        "/log[/{number}[/{trace_id}]] <>",
    ),
    (
        Method::GET,
        "/api/v2/log/2/abc",
        "/log[/{number}[/{trace_id}]] <2> <abc>",
    ),
    (Method::DELETE, "/api/v2/log/2", "/log[/{number}] <2>"),
    (Method::DELETE, "/api/v2/log/2/abc", FALLBACK),
    (
        Method::GET,
        "/api/v2/config/search/plex",
        "/config[/{item}[/{term}]] <search> <plex>",
    ),
    // Fixed text wins over a parameter in the same place.
    (
        Method::GET,
        "/api/v2/plugins/invites/settings",
        "/plugins/invites/settings <>",
    ),
    (
        Method::GET,
        "/api/v2/plugins/invites/ABC123",
        "/plugins/invites/{code} <ABC123>",
    ),
    (
        Method::POST,
        "/api/v2/homepage/overseerr/requests/tv/5/available",
        "/homepage/overseerr/requests/{type}/{id}/available <tv> <5>",
    ),
    // Two patterns of the same shape on different methods.
    (
        Method::POST,
        "/api/v2/homepage/overseerr/requests/tv/5",
        "/homepage/overseerr/requests/{type}/{id}[/{seasons}] <tv> <5>",
    ),
    (
        Method::GET,
        "/api/v2/homepage/overseerr/requests/tv/5",
        "/homepage/overseerr/requests[/{type}[/{limit}[/{offset}]]] <tv> <5>",
    ),
    // A path that exists for another method gets the "not found" answer, as in Slim.
    (Method::POST, "/api/v2/status", FALLBACK),
    (Method::GET, "/api/v2/login", FALLBACK),
    // A path that no route knows.
    (Method::GET, "/api/v2/nothing/here", FALLBACK),
    (Method::GET, "/somewhere/else", FALLBACK),
];

/// The recorded routing cases.
#[derive(serde::Deserialize)]
struct RecordedCases {
    cases: Vec<RecordedCase>,
}

/// One request the recorder sent, and whether the PHP application found a route for it.
#[derive(serde::Deserialize)]
struct RecordedCase {
    request: RecordedRequest,
    reached_a_route: bool,
}

#[derive(serde::Deserialize)]
struct RecordedRequest {
    method: String,
    path: String,
}

fn recorded_routing() -> Vec<RecordedCase> {
    serde_json::from_str::<RecordedCases>(RECORDED_ROUTING)
        .expect("the recorded routing cases are valid JSON")
        .cases
}

#[test]
fn every_organizr_pattern_expands() {
    let routes = organizr_routes();
    assert_eq!(routes.len(), 264, "the fixture holds the full route table");

    for (_, pattern) in &routes {
        if pattern == CATCH_ALL {
            assert_eq!(
                expand(pattern),
                Err(RoutePatternError::MisplacedCatchAll(pattern.clone())),
                "the catch-all pattern becomes the router's fallback"
            );
        } else {
            let expanded = expand(pattern);
            assert!(
                expanded.is_ok(),
                "`{pattern}` does not expand: {expanded:?}"
            );
        }
    }
}

#[tokio::test]
async fn requests_reach_the_route_slim_would_choose() {
    let router = organizr_router();

    for (method, path, expected) in ROUTING_CASES {
        let actual = reach(&router, method.clone(), path).await;
        assert_eq!(actual, *expected, "{method} {path}");
    }
}

#[tokio::test]
async fn router_and_php_agree_on_which_requests_reach_a_route() {
    let router = organizr_router();
    let recorded = recorded_routing();
    assert!(!recorded.is_empty(), "there are recorded routing cases");

    for case in recorded {
        let method = Method::from_bytes(case.request.method.as_bytes())
            .expect("the recorded method is valid");
        let answer = reach(&router, method.clone(), &case.request.path).await;
        assert_eq!(
            answer != FALLBACK,
            case.reached_a_route,
            "{method} {}: the Rust router answered `{answer}`",
            case.request.path
        );
    }
}

#[test]
fn every_routing_expectation_is_backed_by_a_recording() {
    let recorded = recorded_routing();

    for (method, path, _) in ROUTING_CASES {
        // Outside the API the web server answers, not the application.
        if !path.starts_with("/api/v2") {
            continue;
        }
        assert!(
            recorded
                .iter()
                .any(|case| case.request.method == method.as_str() && case.request.path == *path),
            "{method} {path} is not in parity/cases/routing.json; add it to ROUTING in              parity/recorder/record.py and record again"
        );
    }
}

#[tokio::test]
async fn head_request_reaches_the_get_route() {
    let router = organizr_router();
    let request = Request::builder()
        .method(Method::HEAD)
        .uri("/api/v2/status")
        .body(Body::empty())
        .expect("the request is valid");

    let response = router.oneshot(request).await.expect("the router answers");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-length")
            .map(axum::http::HeaderValue::as_bytes),
        Some("/status[/] <>".len().to_string().as_bytes()),
        "the answer describes the GET route's body"
    );
}

#[tokio::test]
async fn cross_site_request_check_works_as_a_layer_on_the_router() {
    let router = organizr_router().layer(CsrfLayer::new());
    let post_login = |headers: &[(&str, &str)]| {
        let mut request = Request::builder()
            .method(Method::POST)
            .uri("/api/v2/login")
            .header("host", "organizr.example.com");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        request.body(Body::empty()).expect("the request is valid")
    };

    // A browser on another site.
    let (status, _) = send(&router, post_login(&[("sec-fetch-site", "cross-site")])).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // A browser on a sibling sub-domain: also another origin.
    let (status, _) = send(&router, post_login(&[("sec-fetch-site", "same-site")])).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // An older browser that sends only `Origin`, from another site.
    let (status, _) = send(
        &router,
        post_login(&[("origin", "https://evil.example.net")]),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // The application's own pages.
    let (status, body) = send(&router, post_login(&[("sec-fetch-site", "same-origin")])).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "/login <>"));

    // An older browser that sends only `Origin`, from the application itself.
    let (status, _) = send(
        &router,
        post_login(&[("origin", "https://organizr.example.com")]),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // A script or an API client: no browser headers at all.
    let (status, _) = send(&router, post_login(&[])).await;
    assert_eq!(status, StatusCode::OK);

    // Reading is never blocked.
    let request = Request::builder()
        .method(Method::GET)
        .uri("/api/v2/status")
        .header("sec-fetch-site", "cross-site")
        .body(Body::empty())
        .expect("the request is valid");
    let (status, _) = send(&router, request).await;
    assert_eq!(status, StatusCode::OK);
}
