//! The device cookie: a random id that a browser keeps for a week.
//!
//! The PHP application gives every caller without the cookie `organizr_user_uuid` a new
//! one, on every route. The id names the device in the forward-auth answer and in the
//! log. It is not a login.

use std::time::{Duration, SystemTime};

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::client::Client;
use crate::cookies::{self, CookieError, CookieScope};

/// The name of the device cookie.
pub const DEVICE_COOKIE: &str = "organizr_user_uuid";

/// How long a browser keeps the device cookie.
const LIFETIME: Duration = Duration::from_hours(7 * 24);

/// Gives a caller without a device cookie a new one. Use it as a middleware.
pub async fn ensure_device_cookie(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let needs_cookie = cookies::cookie_value(headers, DEVICE_COOKIE).is_none();
    // Without an identified client nothing says the browser used HTTPS.
    let secure = request
        .extensions()
        .get::<Client>()
        .is_some_and(|client| client.https);
    let scope = CookieScope::of(headers, secure);

    let mut response = next.run(request).await;

    if needs_cookie {
        match new_device_cookie(&scope, SystemTime::now()) {
            Ok(values) => cookies::append(&mut response, values),
            Err(error) => tracing::error!(%error, "the device cookie could not be written"),
        }
    }
    response
}

/// The `Set-Cookie` values that give a browser a new device id.
pub fn new_device_cookie(
    scope: &CookieScope,
    now: SystemTime,
) -> Result<[HeaderValue; 2], CookieError> {
    let id = Uuid::new_v4().to_string();
    scope.set(DEVICE_COOKIE, &id, now + LIFETIME)
}
