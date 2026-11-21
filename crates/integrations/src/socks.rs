//! The API SOCKS of PHP Organizr: Organizr passes a request on to an application at the
//! address in the settings, and gives back the body of its answer.
//!
//! The caller chooses the method, the path below the address, the query and the body.
//! The caller never chooses the host: the address comes from the settings, and the path
//! cannot climb out of it.

use std::time::Duration;

use reqwest::{Method, Url};
use thiserror::Error;

use crate::http::{Outbound, without_address};

/// How long the application has to answer, as in the PHP application.
pub const TIME_LIMIT: Duration = Duration::from_secs(120);

/// The most bytes of an answer of the application.
pub const ANSWER_LIMIT: usize = 32 * 1024 * 1024;

/// Why a path is refused.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PathError {
    /// The path holds `.` or `..` as a part, an encoded slash, dot or backslash, a
    /// backslash, a control character or bytes that are no UTF-8.
    #[error("the path must not leave the address of the application")]
    Climbs,
    /// The address of the settings is not an address.
    #[error("the address of the application is not valid")]
    Address,
}

/// Why a request failed.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SocksError {
    /// The application could not be reached, or its answer could not be read. The text
    /// names no address.
    #[error("{0}")]
    Unreachable(String),
    /// The answer is larger than [`ANSWER_LIMIT`].
    #[error("the answer of the application is larger than {ANSWER_LIMIT} bytes")]
    TooLarge,
}

/// The value of a hexadecimal digit.
fn hex_value(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The text with each `%xx` read as one byte, as PHP's `rawurldecode` reads it.
fn percent_decoded(text: &str) -> Result<String, PathError> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        let pair = bytes
            .get(index + 1)
            .copied()
            .and_then(hex_value)
            .zip(bytes.get(index + 2).copied().and_then(hex_value));
        if let (b'%', Some((high, low))) = (byte, pair) {
            out.push(high * 16 + low);
            index += 3;
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| PathError::Climbs)
}

/// A part of a path for an address: every byte that may not stand in a part of a path
/// is written as `%` and its two hexadecimal digits.
fn path_part(text: &str) -> String {
    use std::fmt::Write as _;

    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&byte) {
            out.push(char::from(byte));
        } else {
            // Writing into a text cannot fail.
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// The path for the application from the rest of the path of the request, as it came,
/// with its `%xx` codes. The answer starts with a slash.
///
/// As in the PHP application, the codes are read once and empty parts are dropped.
/// A part that is `.` or `..`, a backslash, a control character, or a code that still
/// stands for a dot, a slash or a backslash after the first reading, is refused.
pub fn route_path(raw: &str) -> Result<String, PathError> {
    let decoded = percent_decoded(raw)?;
    let lower = decoded.to_ascii_lowercase();
    if decoded.chars().any(|c| c.is_control() || c == '\\')
        || ["%2e", "%2f", "%5c"]
            .iter()
            .any(|code| lower.contains(code))
    {
        return Err(PathError::Climbs);
    }
    let mut path = String::new();
    for part in decoded.split('/').filter(|part| !part.is_empty()) {
        if part == "." || part == ".." {
            return Err(PathError::Climbs);
        }
        path.push('/');
        path.push_str(&path_part(part));
    }
    if path.is_empty() || decoded.ends_with('/') {
        path.push('/');
    }
    Ok(path)
}

/// The address of the request to the application: the address of the settings, the
/// path from [`route_path`] and the query, which is already encoded.
///
/// As `cleanPath` of the PHP application, a slash at the end goes when there is no
/// query. The answer is checked: it has the scheme, the host and the port of `base`, and
/// its path starts with the path of `base`.
pub fn target(base: &str, path: &str, query: &str) -> Result<String, PathError> {
    let base = base.trim_end_matches('/');
    let parsed_base = Url::parse(base).map_err(|_| PathError::Address)?;
    if parsed_base.cannot_be_a_base() || parsed_base.host_str().is_none() {
        return Err(PathError::Address);
    }
    let mut url = format!("{base}{path}");
    if query.is_empty() {
        while url.ends_with('/') && url.len() > base.len() {
            url.pop();
        }
    } else {
        url.push('?');
        url.push_str(query);
    }
    let parsed = Url::parse(&url).map_err(|_| PathError::Climbs)?;
    let base_path = parsed_base.path().trim_end_matches('/');
    let stays = parsed.scheme() == parsed_base.scheme()
        && parsed.host_str() == parsed_base.host_str()
        && parsed.port_or_known_default() == parsed_base.port_or_known_default()
        && parsed
            .path()
            .strip_prefix(base_path)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
    if stays {
        Ok(url)
    } else {
        Err(PathError::Climbs)
    }
}

/// A request for the application.
#[derive(Debug, Clone)]
pub struct Forward<'a> {
    /// The method.
    pub method: Method,
    /// The address from [`target`].
    pub url: &'a str,
    /// The headers that go to the application.
    pub headers: Vec<(&'static str, String)>,
    /// The body, when one goes to the application.
    pub body: Option<Vec<u8>>,
    /// Whether the certificate of the application is left unchecked.
    pub never_check_certificate: bool,
}

/// What the application answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The status of the answer. The PHP application does not pass it on.
    pub status: u16,
    /// The body of the answer.
    pub body: Vec<u8>,
}

/// Sends the request, without following a redirect, and reads the answer.
pub async fn forward(http: &Outbound, request: Forward<'_>) -> Result<Answer, SocksError> {
    let unreachable = |error| SocksError::Unreachable(without_address(error));
    let mut builder = http
        .direct_client(request.url, request.never_check_certificate)
        .request(request.method, request.url)
        .timeout(TIME_LIMIT);
    for (name, value) in request.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let mut response = builder.send().await.map_err(unreachable)?;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    while let Some(piece) = response.chunk().await.map_err(unreachable)? {
        if body.len() + piece.len() > ANSWER_LIMIT {
            return Err(SocksError::TooLarge);
        }
        body.extend_from_slice(&piece);
    }
    Ok(Answer { status, body })
}

#[cfg(test)]
mod tests {
    use super::{PathError, route_path, target};

    #[test]
    fn path_keeps_its_parts_and_reads_codes_once() {
        assert_eq!(route_path("api/v3/queue").as_deref(), Ok("/api/v3/queue"));
        assert_eq!(route_path("").as_deref(), Ok("/"));
        assert_eq!(route_path("api//v3/").as_deref(), Ok("/api/v3/"));
        assert_eq!(
            route_path("api/v3/series%20one").as_deref(),
            Ok("/api/v3/series%20one")
        );
        assert_eq!(
            route_path("user:pass/jsonrpc").as_deref(),
            Ok("/user:pass/jsonrpc")
        );
        // `?` and `#` in a part stay in the part.
        assert_eq!(route_path("a%3Fb%23c").as_deref(), Ok("/a%3Fb%23c"));
        assert_eq!(route_path("100%25").as_deref(), Ok("/100%25"));
    }

    #[test]
    fn path_that_climbs_is_refused() {
        for raw in [
            "..",
            "api/../../etc",
            "./api",
            "%2e%2e/etc",
            "%2E%2E/etc",
            ".%2e/etc",
            "%252e%252e/etc",
            "api%2f..%2fetc",
            "api%252f",
            "api%5c..",
            "api\\..",
            "api%255c",
            "api%00",
            "api%0d%0aHost:%20other",
            "%ff",
        ] {
            assert_eq!(route_path(raw), Err(PathError::Climbs), "{raw}");
        }
    }

    #[test]
    fn target_stays_below_the_address() {
        let base = "http://sonarr.local:8989/sonarr";
        assert_eq!(
            target(base, "/api/v3/queue", "").as_deref(),
            Ok("http://sonarr.local:8989/sonarr/api/v3/queue")
        );
        assert_eq!(
            target(base, "/api/v3/", "page=2").as_deref(),
            Ok("http://sonarr.local:8989/sonarr/api/v3/?page=2")
        );
        assert_eq!(
            target(base, "/", "").as_deref(),
            Ok("http://sonarr.local:8989/sonarr")
        );
        assert_eq!(
            target("http://sonarr.local:8989/", "/@other.example/x", "").as_deref(),
            Ok("http://sonarr.local:8989/@other.example/x")
        );
        assert_eq!(target("", "/x", ""), Err(PathError::Address));
        assert_eq!(target("http://", "/x", ""), Err(PathError::Address));
    }
}
