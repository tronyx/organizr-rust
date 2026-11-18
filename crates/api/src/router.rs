//! The route table of the API.

use axum::extract::State;
use axum::http::{HeaderValue, Method, Uri, header};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::Response;
use axum::routing::{MethodRouter, delete, get, post};
use axum::{Extension, Router};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::caller::{Caller, identify_caller};
use crate::client::identify_client;
use crate::device_cookie::ensure_device_cookie;
use crate::error::ApiError;
use crate::handlers::{basics, login, logout, register, token, wizard};
use crate::same_origin;
use crate::slim_route::{RoutePatternError, expand};
use crate::state::AppState;

/// The path all API routes start with.
pub const API_BASE: &str = "/api/v2";

/// Builds the router of the application.
///
/// Routes are written as the patterns of the PHP source and expanded for Axum. A request
/// that matches no route, or a route with another method, gets the "not found" answer.
pub fn router(state: AppState) -> Result<Router, RoutePatternError> {
    let routes = Routes::default()
        .route("/", &get(basics::root))?
        .route("/status[/]", &get(basics::status))?
        .route("/ping", &get(basics::ping))?
        .route("/rox", &get(basics::rox))?
        .route("/roxinsocks", &get(basics::roxinsocks))?
        .route("/wizard", &post(wizard::wizard))?
        .route("/login", &post(login::login))?
        .route("/logout", &get(logout::logout))?
        .route("/users/register", &post(register::register))?
        // The PHP router tries `/token/{id}` for a DELETE of the two fixed paths too.
        .route(
            "/token/me",
            &get(token::me).delete(|state, caller| revoke_fixed(state, caller, "me")),
        )?
        .route(
            "/token/validate",
            &post(token::validate).delete(|state, caller| revoke_fixed(state, caller, "validate")),
        )?
        .route("/token/{id}", &delete(token::revoke))?;

    Ok(finish(routes.router, state))
}

/// `DELETE /token/{id}` for a path that Axum gives to a route without a parameter.
async fn revoke_fixed(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    id: &'static str,
) -> Result<Response, ApiError> {
    token::revoke_by_text(&state, &caller, id).await
}

/// Adds what every route gets, and the answer for a request that no route takes.
fn finish(routes: Router<AppState>, state: AppState) -> Router {
    routes
        // Route layers run only when a route takes the request, so a path that is not a
        // route, or not one for the method, stays "not found" whoever asks, as in the PHP
        // application.
        .route_layer(same_origin::layer())
        .route_layer(from_fn(same_origin::answer_refusal))
        .fallback(not_found)
        .method_not_allowed_fallback(not_found)
        .layer(from_fn(ensure_device_cookie))
        // Added before the layer that identifies the client, so it runs after it: the
        // cookie it may drop needs to know whether the browser used HTTPS.
        .layer(from_fn_with_state(state.clone(), identify_caller))
        // Added after the device cookie, so it runs before it: the device cookie reads
        // the client that this layer works out.
        .layer(from_fn_with_state(state.clone(), identify_client))
        // The PHP application starts a PHP session on every request, which adds these
        // three headers to every answer. They also keep answers out of caches.
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store, no-cache, must-revalidate"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::PRAGMA,
            HeaderValue::from_static("no-cache"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::EXPIRES,
            HeaderValue::from_static("Thu, 19 Nov 1981 08:52:00 GMT"),
        ))
        .with_state(state)
}

/// Collects routes that are written as patterns of the PHP source.
#[derive(Default)]
struct Routes {
    router: Router<AppState>,
}

impl Routes {
    /// Adds every explicit route a pattern stands for, under [`API_BASE`].
    fn route(
        mut self,
        pattern: &str,
        methods: &MethodRouter<AppState>,
    ) -> Result<Self, RoutePatternError> {
        for route in expand(pattern)? {
            let path = format!("{API_BASE}{}", route.path());
            self.router = self.router.route(&path, methods.clone());
        }
        Ok(self)
    }
}

/// The answer to a request that no route takes.
async fn not_found(method: Method, uri: Uri) -> ApiError {
    ApiError::NotFound {
        endpoint: uri.path().to_owned(),
        method: method.as_str().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode, header};
    use axum::routing::get;
    use http_body_util::BodyExt;
    use organizr_db::Database;
    use organizr_settings::Settings;
    use tower::ServiceExt;

    use super::{Routes, finish};
    use crate::state::AppState;

    /// What the PHP application answers when the form key is missing or wrong.
    const REFUSAL: &str = "{\n    \"response\": {\n        \"result\": \"error\",\n        \"message\": \"Not authorized for current Route: /api/v2/change\",\n        \"data\": null\n    }\n}";

    /// The layers of the real router around one route that reads and one that changes.
    /// The application itself has no route that changes something yet.
    async fn app() -> (Router, tempfile::TempDir) {
        let directory = tempfile::tempdir().expect("a temporary folder can be created");
        let file = directory.path().join("organizr.db");
        let url = format!("sqlite:{}", file.display().to_string().replace('\\', "/"));
        let database = Database::connect(&url).await.expect("the database opens");

        let routes = Routes::default()
            .route(
                "/change",
                &get(|| async { "read" })
                    .post(|| async { "changed" })
                    .put(|| async { "changed" })
                    .delete(|| async { "changed" }),
            )
            .expect("the pattern is valid");
        let state = AppState::new(database, Settings::defaults());
        (finish(routes.router, state), directory)
    }

    async fn send(app: &Router, method: Method, headers: &[(&str, &str)]) -> (StatusCode, String) {
        let mut request = Request::builder()
            .method(method)
            .uri("/api/v2/change")
            .header(header::HOST, "organizr.example.com");
        for (name, value) in headers {
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
    async fn request_from_another_site_that_would_change_something_is_refused() {
        let (app, _directory) = app().await;
        let from_elsewhere: [&[(&str, &str)]; 3] = [
            // A current browser on another site.
            &[("sec-fetch-site", "cross-site")],
            // A current browser on a sibling sub-domain, for example another application
            // behind the same proxy.
            &[("sec-fetch-site", "same-site")],
            // An older browser, which only says where the page came from.
            &[("origin", "https://evil.example.net")],
        ];

        for headers in from_elsewhere {
            for method in [Method::POST, Method::PUT, Method::DELETE] {
                let (status, body) = send(&app, method.clone(), headers).await;

                assert_eq!(
                    status,
                    StatusCode::UNAUTHORIZED,
                    "{method} with {headers:?}"
                );
                assert_eq!(body, REFUSAL, "{method} with {headers:?}");
            }
        }
    }

    #[tokio::test]
    async fn request_from_the_application_itself_changes_something() {
        let (app, _directory) = app().await;
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
            let (status, body) = send(&app, Method::POST, headers).await;

            assert_eq!(
                (status, body.as_str()),
                (StatusCode::OK, "changed"),
                "{headers:?}"
            );
        }
    }

    #[tokio::test]
    async fn reading_from_another_site_is_not_refused() {
        let (app, _directory) = app().await;

        let (status, body) = send(&app, Method::GET, &[("sec-fetch-site", "cross-site")]).await;

        assert_eq!((status, body.as_str()), (StatusCode::OK, "read"));
    }

    #[tokio::test]
    async fn refusal_carries_the_headers_of_every_answer() {
        let (app, _directory) = app().await;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/api/v2/change")
            .header(header::HOST, "organizr.example.com")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .expect("the request is valid");

        let response = app.oneshot(request).await.expect("the router answers");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"application/json;charset=UTF-8".as_slice())
        );
        assert_eq!(
            response
                .headers()
                .get(header::CACHE_CONTROL)
                .map(axum::http::HeaderValue::as_bytes),
            Some(b"no-store, no-cache, must-revalidate".as_slice())
        );
    }
}
