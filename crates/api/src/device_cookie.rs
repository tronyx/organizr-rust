//! The device cookie: a random id that a browser keeps for a week.
//!
//! The PHP application gives every caller without the cookie `organizr_user_uuid` a new
//! one, on every route. The id names the device in the forward-auth answer and in the
//! log. It is not a login.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

use crate::client::Client;
use crate::cookie_domain::cookie_domains;

/// The name of the device cookie.
pub const DEVICE_COOKIE: &str = "organizr_user_uuid";

/// How long a browser keeps the device cookie.
const LIFETIME: Duration = Duration::from_hours(7 * 24);

/// Gives a caller without a device cookie a new one. Use it as a middleware.
pub async fn ensure_device_cookie(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let needs_cookie = !has_cookie(headers, DEVICE_COOKIE);
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    // Without an identified client nothing says the browser used HTTPS.
    let secure = request
        .extensions()
        .get::<Client>()
        .is_some_and(|client| client.https);

    let mut response = next.run(request).await;

    if needs_cookie {
        let expires = SystemTime::now() + LIFETIME;
        let id = Uuid::new_v4().to_string();
        match set_cookie_values(DEVICE_COOKIE, &id, expires, &host, secure) {
            Ok(values) => {
                for value in values {
                    response.headers_mut().append(header::SET_COOKIE, value);
                }
            }
            Err(error) => tracing::error!(%error, "the device cookie could not be written"),
        }
    }
    response
}

/// Whether the request carries a cookie of the given name, with any value.
fn has_cookie(headers: &HeaderMap, name: &str) -> bool {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|cookies| cookies.split(';'))
        .any(|cookie| cookie.trim().split('=').next() == Some(name))
}

/// Why a cookie could not be written.
#[derive(Debug, thiserror::Error)]
enum CookieError {
    /// The expiry time is outside what a date can hold.
    #[error("the expiry time of the cookie is not a valid date")]
    Expiry,
    /// The cookie holds a character that a header cannot carry.
    #[error("the cookie holds a character that is not allowed in a header")]
    Header,
}

/// The two `Set-Cookie` values for one cookie, as the PHP application sends them: the
/// first for the wider domain, the second for the host itself.
///
/// A host without a wider domain (an IP address, `localhost`) gets no domain on either.
/// `Secure` and `HttpOnly` are sent only over HTTPS, as PHP does. Whether the browser
/// used HTTPS is what a trusted proxy reports; see [`Client`]. `SameSite=Lax` replaces
/// PHP's `SameSite=None`.
fn set_cookie_values(
    name: &str,
    value: &str,
    expires: SystemTime,
    host_header: &str,
    secure: bool,
) -> Result<[HeaderValue; 2], CookieError> {
    let since_epoch = expires
        .duration_since(UNIX_EPOCH)
        .map_err(|_| CookieError::Expiry)?;
    let seconds = i64::try_from(since_epoch.as_secs()).map_err(|_| CookieError::Expiry)?;
    let expires = jiff::Timestamp::from_second(seconds)
        .map_err(|_| CookieError::Expiry)?
        .strftime("%a, %d-%b-%Y %H:%M:%S GMT");

    let domains = cookie_domains(host_header);
    let render = |domain: &str| {
        let mut cookie = format!("{name}={value}; expires={expires}; path=/");
        if !domains.wide.is_empty() {
            cookie.push_str("; domain=");
            cookie.push_str(domain);
        }
        cookie.push_str("; SameSite=Lax");
        if secure {
            cookie.push_str("; Secure; HttpOnly");
        }
        HeaderValue::from_str(&cookie).map_err(|_| CookieError::Header)
    };

    Ok([render(&domains.wide)?, render(&domains.host)?])
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{has_cookie, set_cookie_values};

    /// 2026-11-25 17:16:08 UTC, the expiry in a header that the PHP application sent.
    const EXPIRY_SECONDS: u64 = 1_795_626_968;

    fn values(host: &str, secure: bool) -> [String; 2] {
        let expires = UNIX_EPOCH + Duration::from_secs(EXPIRY_SECONDS);
        set_cookie_values("organizr_user_uuid", "the-id", expires, host, secure)
            .expect("the cookie can be written")
            .map(|value| value.to_str().expect("the value is text").to_owned())
    }

    fn headers(name: header::HeaderName, value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn cookie_is_set_for_the_wider_domain_and_for_the_host() {
        assert_eq!(
            values("app.example.com", false),
            [
                "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=.example.com; SameSite=Lax",
                "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=app.example.com; SameSite=Lax",
            ]
        );
    }

    #[test]
    fn cookie_over_https_is_secure_and_hidden_from_scripts() {
        assert_eq!(
            values("organizr.test", true),
            [
                "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=.organizr.test; SameSite=Lax; Secure; HttpOnly",
                "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=organizr.test; SameSite=Lax; Secure; HttpOnly",
            ]
        );
    }

    #[test]
    fn host_without_a_wider_domain_gets_no_domain_at_all() {
        for host in ["127.0.0.1:8098", "localhost"] {
            assert_eq!(
                values(host, false),
                [
                    "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; SameSite=Lax",
                    "organizr_user_uuid=the-id; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; SameSite=Lax",
                ],
                "host `{host}`"
            );
        }
    }

    #[test]
    fn cookie_is_found_by_name_among_others() {
        let request = headers(
            header::COOKIE,
            "theme=dark; organizr_user_uuid=abc; other=1",
        );

        assert!(has_cookie(&request, "organizr_user_uuid"));
        assert!(has_cookie(&request, "theme"));
        assert!(!has_cookie(&request, "organizr_user"));
        assert!(!has_cookie(&request, "uuid"));
        assert!(!has_cookie(&HeaderMap::new(), "organizr_user_uuid"));
    }

    #[test]
    fn cookie_with_an_empty_value_still_counts() {
        let request = headers(header::COOKIE, "organizr_user_uuid=");

        assert!(has_cookie(&request, "organizr_user_uuid"));
    }
}
