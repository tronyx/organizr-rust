//! The request id (D-13): a random id for each request, made by the server.
//!
//! The id goes back in the header `X-Request-Id` of the answer, and every line of the log
//! that the request writes carries it in the field `request_id` of the span `request`.
//! An `X-Request-Id` header of the client is never used, so a client cannot put text of
//! its own into the log.

use std::sync::Arc;

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument as _;

/// The header of the request id.
pub const HEADER: HeaderName = HeaderName::from_static("x-request-id");

/// The id of a request, as the server made it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(Arc<str>);

impl RequestId {
    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Gives the request an id, runs it in the span `request`, and puts the id into the
/// header of the answer.
///
/// The layer can stand twice, around the web files and around the API. The inner one
/// then keeps the id of the outer one.
pub async fn assign(mut request: Request, next: Next) -> Response {
    if request.extensions().get::<RequestId>().is_some() {
        return next.run(request).await;
    }
    let id: Arc<str> = uuid::Uuid::new_v4().to_string().into();
    request.headers_mut().remove(HEADER);
    request.extensions_mut().insert(RequestId(Arc::clone(&id)));
    let span = tracing::info_span!("request", request_id = %id);
    let mut response = next.run(request).instrument(span).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(HEADER, value);
    }
    response
}
