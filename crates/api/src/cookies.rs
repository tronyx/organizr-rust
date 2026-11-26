//! Reading the cookies of a request and writing cookies as the PHP application does.

use std::fmt::Write as _;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::Response;

use crate::cookie_domain::cookie_domains;

/// How far in the past the expiry of a deleted cookie lies.
const DELETED_SINCE: Duration = Duration::from_hours(1);

/// Why a cookie could not be written.
#[derive(Debug, thiserror::Error)]
pub enum CookieError {
    /// The expiry time is outside what a date can hold.
    #[error("the expiry time of the cookie is not a valid date")]
    Expiry,
    /// The cookie holds a character that a header cannot carry.
    #[error("the cookie holds a character that is not allowed in a header")]
    Header,
}

/// Where a cookie goes: the host the browser asked for, and whether it used HTTPS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookieScope {
    /// The `Host` header of the request.
    pub host: String,
    /// Whether the browser used HTTPS. See [`Client`](crate::client::Client).
    pub secure: bool,
}

impl CookieScope {
    /// The scope for the answer to a request with the given headers.
    pub fn of(headers: &HeaderMap, secure: bool) -> Self {
        let host = headers
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        Self { host, secure }
    }

    /// The two `Set-Cookie` values that give the browser a cookie until `expires`.
    ///
    /// The PHP application sends every cookie twice: first for the wider domain, then
    /// for the host itself. A host without a wider domain (an IP address, `localhost`)
    /// gets no domain on either. `Secure` and `HttpOnly` are sent only over HTTPS, as
    /// PHP does. `SameSite=Lax` replaces PHP's `SameSite=None`.
    pub fn set(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
    ) -> Result<[HeaderValue; 2], CookieError> {
        self.render(name, value, expires, self.secure)
    }

    /// As [`set`](Self::set), for a cookie that no script of the page may read: it is
    /// `HttpOnly` also over plain HTTP. The session cookie is such a cookie.
    pub fn set_hidden(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
    ) -> Result<[HeaderValue; 2], CookieError> {
        self.render(name, value, expires, true)
    }

    /// The two `Set-Cookie` values for a cookie that another application reads: the
    /// session that a single sign-on got from it. The value is written with `%xx` for
    /// every character that is no letter, digit, `-`, `_`, `.` or `~`, as PHP writes it.
    /// `path` is the path of the application on its host. `http_only` hides the cookie
    /// from scripts, over HTTPS and over plain HTTP.
    pub fn set_for_application(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
        http_only: bool,
        path: &str,
    ) -> Result<[HeaderValue; 2], CookieError> {
        let mut encoded = String::with_capacity(value.len());
        for byte in value.bytes() {
            if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
                encoded.push(char::from(byte));
            } else {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
        // A cookie that the application wants hidden from scripts stays hidden over plain
        // HTTP too. PHP sends `HttpOnly` only over HTTPS.
        self.render_at(name, &encoded, expires, http_only, path)
    }

    fn render(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
        http_only: bool,
    ) -> Result<[HeaderValue; 2], CookieError> {
        self.render_at(name, value, expires, http_only, "/")
    }

    fn render_at(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
        http_only: bool,
        path: &str,
    ) -> Result<[HeaderValue; 2], CookieError> {
        let since_epoch = expires
            .duration_since(UNIX_EPOCH)
            .map_err(|_| CookieError::Expiry)?;
        let seconds = i64::try_from(since_epoch.as_secs()).map_err(|_| CookieError::Expiry)?;
        let expires = jiff::Timestamp::from_second(seconds)
            .map_err(|_| CookieError::Expiry)?
            .strftime("%a, %d-%b-%Y %H:%M:%S GMT");

        let domains = cookie_domains(&self.host);
        let render = |domain: &str| {
            let mut cookie = format!("{name}={value}; expires={expires}; path={path}");
            if !domains.wide.is_empty() {
                cookie.push_str("; domain=");
                cookie.push_str(domain);
            }
            cookie.push_str("; SameSite=Lax");
            if self.secure {
                cookie.push_str("; Secure");
            }
            if http_only {
                cookie.push_str("; HttpOnly");
            }
            HeaderValue::from_str(&cookie).map_err(|_| CookieError::Header)
        };

        Ok([render(&domains.wide)?, render(&domains.host)?])
    }

    /// The two `Set-Cookie` values that make the browser drop a cookie: an empty value
    /// and an expiry one hour before `now`, as the PHP application sends them.
    pub fn delete(&self, name: &str, now: SystemTime) -> Result<[HeaderValue; 2], CookieError> {
        let expired = now.checked_sub(DELETED_SINCE).ok_or(CookieError::Expiry)?;
        self.set(name, "", expired)
    }

    /// The `Set-Cookie` values of the session cookie: the cookie itself at `/`, and the
    /// deletion of cookies of the same name at the paths of the API. A host that shares
    /// the cookie domain could plant such a cookie, which the browser sends first
    /// (D-145).
    pub fn set_session(
        &self,
        name: &str,
        value: &str,
        expires: SystemTime,
    ) -> Result<Vec<HeaderValue>, CookieError> {
        // The deletions come first, so a client that keeps one cookie per name, whatever
        // its path, ends up with the session.
        let mut values = self.drop_shadows(name, SystemTime::now())?;
        values.extend(self.set_hidden(name, value, expires)?);
        Ok(values)
    }

    /// The `Set-Cookie` values that make the browser drop the session cookie, at `/` and
    /// at the paths of the API (D-145).
    pub fn delete_session(
        &self,
        name: &str,
        now: SystemTime,
    ) -> Result<Vec<HeaderValue>, CookieError> {
        let mut values = self.drop_shadows(name, now)?;
        values.extend(self.delete_hidden(name, now)?);
        Ok(values)
    }

    /// The deletion of cookies of the name at the paths that a planted cookie would use
    /// to come before the one at `/`.
    fn drop_shadows(&self, name: &str, now: SystemTime) -> Result<Vec<HeaderValue>, CookieError> {
        let expired = now.checked_sub(DELETED_SINCE).ok_or(CookieError::Expiry)?;
        let mut values = Vec::new();
        for path in SHADOW_PATHS {
            values.extend(self.render_at(name, "", expired, true, path)?);
        }
        Ok(values)
    }

    /// As [`delete`](Self::delete), for a cookie that was set with
    /// [`set_hidden`](Self::set_hidden).
    pub fn delete_hidden(
        &self,
        name: &str,
        now: SystemTime,
    ) -> Result<[HeaderValue; 2], CookieError> {
        let expired = now.checked_sub(DELETED_SINCE).ok_or(CookieError::Expiry)?;
        self.set_hidden(name, "", expired)
    }
}

/// The paths of the API, where a cookie planted by another host of the cookie domain
/// would come before the session cookie at `/` (D-145).
const SHADOW_PATHS: [&str; 2] = ["/api", "/api/v2"];

/// Adds `Set-Cookie` values to an answer, after the ones it already has.
pub fn append(response: &mut Response, values: impl IntoIterator<Item = HeaderValue>) {
    for value in values {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

/// Adds `Set-Cookie` values to an answer, before the ones it already has. A browser
/// applies the values in order, so for a cookie that is set twice the later one stays.
pub fn prepend(response: &mut Response, values: impl IntoIterator<Item = HeaderValue>) {
    let later: Vec<HeaderValue> = response
        .headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .cloned()
        .collect();
    response.headers_mut().remove(header::SET_COOKIE);
    append(response, values);
    append(response, later);
}

/// The cookies of a request, in the order the browser sent them: name and value.
pub fn request_cookies(headers: &HeaderMap) -> impl Iterator<Item = (&str, &str)> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|cookies| cookies.split(';'))
        .map(|cookie| {
            let cookie = cookie.trim();
            cookie.split_once('=').unwrap_or((cookie, ""))
        })
        .filter(|(name, _)| !name.is_empty())
}

/// The values of the cookies of the given name that the request carries, in the order
/// of the request. A browser sends a cookie with a longer path first.
pub fn cookie_values<'a>(headers: &'a HeaderMap, name: &str) -> impl Iterator<Item = &'a str> {
    request_cookies(headers)
        .filter(move |(cookie, _)| *cookie == name)
        .map(|(_, value)| value)
}

/// The value of the first cookie of the given name that the request carries.
pub fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    request_cookies(headers)
        .find(|(cookie, _)| *cookie == name)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use axum::http::{HeaderMap, HeaderValue, header};

    use super::{CookieScope, cookie_value, request_cookies};

    /// 2026-11-25 17:16:08 UTC, the expiry in a header that the PHP application sent.
    const EXPIRY_SECONDS: u64 = 1_795_626_968;

    fn expiry() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(EXPIRY_SECONDS)
    }

    fn scope(host: &str, secure: bool) -> CookieScope {
        CookieScope {
            host: host.to_owned(),
            secure,
        }
    }

    #[test]
    fn cookie_of_another_application_has_its_path_and_an_encoded_value() {
        let over_https = text(
            scope("organizr.example.com", true)
                .set_for_application("connect.sid", "s:abc.def+g/h=", expiry(), true, "/tautulli")
                .expect("the cookie can be written"),
        );

        assert_eq!(
            over_https[0],
            "connect.sid=s%3Aabc.def%2Bg%2Fh%3D; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/tautulli; domain=.example.com; SameSite=Lax; Secure; HttpOnly"
        );
        assert!(over_https[1].contains("; domain=organizr.example.com;"));

        // Not hidden from scripts when the application did not ask for that. A cookie
        // that is hidden stays hidden over plain HTTP; PHP drops `HttpOnly` there.
        let readable = text(
            scope("organizr.example.com", true)
                .set_for_application("mpt", "token-1_2~3", expiry(), false, "/")
                .expect("the cookie can be written"),
        );
        assert!(readable[0].starts_with("mpt=token-1_2~3; "));
        assert!(!readable[0].contains("HttpOnly"));
        let plain = text(
            scope("organizr.example.com", false)
                .set_for_application("tautulli_token_x", "v", expiry(), true, "/")
                .expect("the cookie can be written"),
        );
        assert!(
            plain[0].contains("; HttpOnly") && !plain[0].contains("Secure"),
            "{}",
            plain[0]
        );
    }

    fn text(values: [HeaderValue; 2]) -> [String; 2] {
        values.map(|value| value.to_str().expect("the value is text").to_owned())
    }

    fn values(host: &str, secure: bool) -> [String; 2] {
        text(
            scope(host, secure)
                .set("organizr_user_uuid", "the-id", expiry())
                .expect("the cookie can be written"),
        )
    }

    fn headers(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static(value));
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
    fn hidden_cookie_is_hidden_from_scripts_on_every_connection() {
        let over_http = scope("app.example.com", false);
        let over_https = scope("app.example.com", true);
        let set = |scope: &super::CookieScope| {
            text(
                scope
                    .set_hidden("organizr_token_temp", "the-token", expiry())
                    .expect("the cookie can be written"),
            )
        };

        assert_eq!(
            set(&over_http),
            [
                "organizr_token_temp=the-token; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=.example.com; SameSite=Lax; HttpOnly",
                "organizr_token_temp=the-token; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=app.example.com; SameSite=Lax; HttpOnly",
            ]
        );
        assert_eq!(
            set(&over_https),
            [
                "organizr_token_temp=the-token; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=.example.com; SameSite=Lax; Secure; HttpOnly",
                "organizr_token_temp=the-token; expires=Wed, 25-Nov-2026 17:16:08 GMT; path=/; domain=app.example.com; SameSite=Lax; Secure; HttpOnly",
            ]
        );
        let deleted = text(
            over_http
                .delete_hidden("organizr_token_temp", expiry())
                .expect("the cookie can be written"),
        );
        assert_eq!(
            deleted[0],
            "organizr_token_temp=; expires=Wed, 25-Nov-2026 16:16:08 GMT; path=/; domain=.example.com; SameSite=Lax; HttpOnly"
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
    fn deleted_cookie_is_empty_and_expired_an_hour_ago() {
        let deleted = scope("app.example.com", true)
            .delete("organizr_token_temp", expiry())
            .expect("the cookie can be written");

        assert_eq!(
            text(deleted),
            [
                "organizr_token_temp=; expires=Wed, 25-Nov-2026 16:16:08 GMT; path=/; domain=.example.com; SameSite=Lax; Secure; HttpOnly",
                "organizr_token_temp=; expires=Wed, 25-Nov-2026 16:16:08 GMT; path=/; domain=app.example.com; SameSite=Lax; Secure; HttpOnly",
            ]
        );
    }

    #[test]
    fn cookie_that_a_header_cannot_carry_is_refused() {
        let result = scope("app.example.com", false).set("name", "line\nbreak", expiry());

        assert!(result.is_err(), "{result:?}");
    }

    #[test]
    fn scope_takes_the_host_from_the_request() {
        let mut request = HeaderMap::new();
        request.insert(
            header::HOST,
            HeaderValue::from_static("app.example.com:8443"),
        );

        assert_eq!(
            CookieScope::of(&request, true),
            scope("app.example.com:8443", true)
        );
        assert_eq!(CookieScope::of(&HeaderMap::new(), false), scope("", false));
    }

    #[test]
    fn prepended_cookies_come_before_the_ones_of_the_answer() {
        use axum::response::IntoResponse;

        let mut response = ().into_response();
        let cookie = |text: &'static str| HeaderValue::from_static(text);
        super::append(&mut response, [cookie("a=1"), cookie("b=2")]);

        super::prepend(&mut response, [cookie("a=; deleted"), cookie("c=3")]);

        let values: Vec<&str> = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .map(|value| value.to_str().expect("the value is text"))
            .collect();
        assert_eq!(values, ["a=; deleted", "c=3", "a=1", "b=2"]);
    }

    #[test]
    fn cookies_of_a_request_are_read_in_order() {
        let request = headers("theme=dark; organizr_user_uuid=abc;other=1=2; flag; =nameless");

        let cookies: Vec<(&str, &str)> = request_cookies(&request).collect();

        assert_eq!(
            cookies,
            [
                ("theme", "dark"),
                ("organizr_user_uuid", "abc"),
                ("other", "1=2"),
                ("flag", ""),
            ]
        );
    }

    #[test]
    fn cookie_is_found_by_its_whole_name() {
        let request = headers("theme=dark; organizr_user_uuid=abc; empty=; theme=light");

        assert_eq!(cookie_value(&request, "organizr_user_uuid"), Some("abc"));
        assert_eq!(cookie_value(&request, "theme"), Some("dark"));
        assert_eq!(cookie_value(&request, "empty"), Some(""));
        assert_eq!(cookie_value(&request, "organizr_user"), None);
        assert_eq!(cookie_value(&request, "uuid"), None);
        assert_eq!(cookie_value(&HeaderMap::new(), "theme"), None);
    }
}
