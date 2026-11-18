//! The route table of the API.

use axum::Router;
use axum::http::{HeaderValue, Method, Uri, header};
use axum::middleware::from_fn;
use axum::routing::{MethodRouter, get};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::device_cookie::ensure_device_cookie;
use crate::error::ApiError;
use crate::handlers::basics;
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
        .route("/roxinsocks", &get(basics::roxinsocks))?;

    Ok(routes
        .router
        .fallback(not_found)
        .method_not_allowed_fallback(not_found)
        .layer(from_fn(ensure_device_cookie))
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
        .with_state(state))
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
