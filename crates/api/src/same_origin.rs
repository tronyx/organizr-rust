//! Refuses a request from another site that would change something.
//!
//! A browser tells the server where a request comes from: current browsers in the
//! `Sec-Fetch-Site` header, older ones in `Origin`. A request that changes something
//! (any method other than GET, HEAD and OPTIONS) is let through when it comes from a
//! page of the application itself, or from a client that is not a browser and sends
//! neither header. `tower_http::csrf` makes that decision.
//!
//! This replaces the "form key" of the PHP application, which was the same for every
//! visitor, never changed and was checked on POST only. A refused request gets the answer
//! PHP gives for a wrong form key, so clients see the error they know.

use axum::extract::Request;
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use tower_http::csrf::{CsrfLayer, ProtectionError};

use crate::client::Client;
use crate::error::ApiError;

/// The layer that makes the decision. A refused request leaves it as an empty 403 that
/// carries the reason; [`answer_refusal`] must wrap it to give the refusal its answer.
pub fn layer() -> CsrfLayer {
    CsrfLayer::new()
}

/// Turns a refusal of [`layer`] into the answer of the API. Use it as a middleware around
/// that layer.
pub async fn answer_refusal(request: Request, next: Next) -> Response {
    let route = request.uri().path().to_owned();
    let method = request.method().clone();
    let client = request.extensions().get::<Client>().copied();
    let came_from = request
        .headers()
        .get(header::ORIGIN)
        .or_else(|| request.headers().get(header::REFERER))
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);

    let response = next.run(request).await;

    match response.extensions().get::<ProtectionError>() {
        Some(reason) => {
            tracing::warn!(
                %method,
                %route,
                %reason,
                came_from = came_from.as_deref().unwrap_or("unknown"),
                client = ?client.map(|client| client.address),
                "refused a request from another site"
            );
            ApiError::CrossSite { route }.into_response()
        }
        None => response,
    }
}
