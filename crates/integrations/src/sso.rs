//! Single sign-on: a user who logs in to Organizr is logged in to other applications
//! too.
//!
//! At a login, Organizr asks each application for a session of the user, with the
//! password or the Plex token of the login. It hands the result to the browser as the
//! cookie that the application itself would set. The applications must be on the same
//! domain as Organizr, so that the browser sends them the cookie.

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::http::{Outbound, without_address};

/// Who logs in, and from where.
#[derive(Clone, Copy)]
pub struct Login<'a> {
    /// The user name.
    pub username: &'a str,
    /// The e-mail address.
    pub email: &'a str,
    /// The password of the login. Empty for a login with a Plex token.
    pub password: &'a str,
    /// The Plex token of the user, when the login brought one.
    pub plex_token: Option<&'a str>,
    /// The address of the browser, for the log of the application.
    pub client_ip: &'a str,
    /// The name that the browser gives itself.
    pub user_agent: Option<&'a str>,
}

/// An account of the application that is used when the application does not know the
/// user: the "fallback" settings.
#[derive(Clone, Copy)]
pub struct Fallback<'a> {
    /// The name, or for Overseerr the e-mail address.
    pub user: &'a str,
    /// The password.
    pub password: &'a str,
}

/// A session of Tautulli.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TautulliSession {
    /// The id of the Tautulli install, which is a part of the name of its cookie.
    pub uuid: String,
    /// The value of the cookie.
    pub token: String,
    /// The path of the Tautulli install, which is the path of its cookie.
    pub path: String,
}

#[derive(Deserialize)]
struct OmbiAnswer {
    access_token: Option<String>,
}

/// Asks Ombi at `url` for a token: with the Plex token of the login when there is one,
/// else with the name and the password. Returns `None` when Ombi gives none.
pub async fn ombi_token(
    http: &Outbound,
    url: &str,
    login: Login<'_>,
    fallback: Option<Fallback<'_>>,
    timeout: Duration,
) -> Option<String> {
    let token = ombi_request(http, url, login.username, login.password, login, timeout).await;
    match (token, fallback) {
        (Some(token), _) => Some(token),
        (None, Some(fallback)) => {
            tracing::warn!("Ombi gave no token; trying the fallback account");
            let without_plex = Login {
                plex_token: None,
                ..login
            };
            ombi_request(
                http,
                url,
                fallback.user,
                fallback.password,
                without_plex,
                timeout,
            )
            .await
        }
        (None, None) => {
            tracing::warn!("Ombi gave no token");
            None
        }
    }
}

async fn ombi_request(
    http: &Outbound,
    url: &str,
    username: &str,
    password: &str,
    login: Login<'_>,
    timeout: Duration,
) -> Option<String> {
    let (path, body) = match login.plex_token {
        Some(token) => (
            "/api/v1/Token/plextoken",
            json!({"username": "", "password": "", "rememberMe": "true", "plexToken": token}),
        ),
        None => (
            "/api/v1/Token",
            json!({"username": username, "password": password, "rememberMe": "true", "plexToken": null}),
        ),
    };
    let target = format!("{url}{path}");
    let response = http
        .client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("Accept", "application/json")
        .header("X-Forwarded-For", login.client_ip)
        .json(&body)
        .send()
        .await
        // The address can hold a key or a password, so the log names none.
        .map_err(|error| tracing::warn!(error = %without_address(error), "Ombi cannot be reached"))
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let answer: OmbiAnswer = response.json().await.ok()?;
    answer.access_token.filter(|token| !token.is_empty())
}

/// Asks Overseerr at `url` for a session and returns the value of its `connect.sid`
/// cookie. Returns `None` when Overseerr gives none.
pub async fn overseerr_session(
    http: &Outbound,
    url: &str,
    login: Login<'_>,
    fallback: Option<Fallback<'_>>,
    timeout: Duration,
) -> Option<String> {
    let session = overseerr_request(http, url, login.email, login.password, login, timeout).await;
    match (session, fallback) {
        (Some(session), _) => Some(session),
        (None, Some(fallback)) => {
            tracing::warn!("Overseerr gave no session; trying the fallback account");
            let without_plex = Login {
                plex_token: None,
                ..login
            };
            overseerr_request(
                http,
                url,
                fallback.user,
                fallback.password,
                without_plex,
                timeout,
            )
            .await
        }
        (None, None) => {
            tracing::warn!("Overseerr gave no session");
            None
        }
    }
}

async fn overseerr_request(
    http: &Outbound,
    url: &str,
    email: &str,
    password: &str,
    login: Login<'_>,
    timeout: Duration,
) -> Option<String> {
    let (path, body) = match login.plex_token {
        Some(token) => (
            "/api/v1/auth/plex",
            json!({"email": "", "password": "", "authToken": token}),
        ),
        None => (
            "/api/v1/auth/local",
            json!({"email": email, "password": password, "authToken": null}),
        ),
    };
    let target = format!("{url}{path}");
    let response = http
        .client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("X-Forwarded-For", login.client_ip)
        .json(&body)
        .send()
        .await
        // The address can hold a key or a password, so the log names none.
        .map_err(
            |error| tracing::warn!(error = %without_address(error), "Overseerr cannot be reached"),
        )
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|cookie| cookie_value(cookie, "connect.sid"))
}

/// The value of a cookie in a `Set-Cookie` header, with its `%xx` parts decoded.
fn cookie_value(header: &str, name: &str) -> Option<String> {
    let (found, value) = header.split(';').next()?.split_once('=')?;
    if found.trim() != name || value.is_empty() {
        return None;
    }
    Some(percent_decoded(value.trim()))
}

fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(byte) = bytes.get(index) {
        let hex = bytes
            .get(index + 1..index + 3)
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        let (value, length) = match hex {
            Some(value) if *byte == b'%' => (value, 3),
            _ => (*byte, 1),
        };
        decoded.push(value);
        index += length;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[derive(Deserialize)]
struct TautulliAnswer {
    token: Option<String>,
    uuid: Option<String>,
}

/// Asks each Tautulli install for a session. `installs` holds the address of each one
/// with the path of its cookie. An install that gives no session is left out.
pub async fn tautulli_sessions(
    http: &Outbound,
    installs: &[(String, String)],
    login: Login<'_>,
    timeout: Duration,
) -> Vec<TautulliSession> {
    let mut sessions = Vec::new();
    for (url, path) in installs {
        let target = format!("{url}/auth/signin");
        let form: [(&str, &str); 4] = match login.plex_token {
            Some(token) => [
                ("username", ""),
                ("password", ""),
                ("token", token),
                ("remember_me", "1"),
            ],
            None => [
                ("username", login.username),
                ("password", login.password),
                ("token", ""),
                ("remember_me", "1"),
            ],
        };
        let mut request = http
            .client_for(&target)
            .post(&target)
            .timeout(timeout)
            .header("Accept", "application/json")
            .header("X-Forwarded-For", login.client_ip)
            .form(&form);
        if let Some(user_agent) = login.user_agent {
            request = request.header("User-Agent", user_agent);
        }
        let answer = match request.send().await {
            Ok(response) if response.status().is_success() => {
                response.json::<TautulliAnswer>().await.ok()
            }
            Ok(_) => None,
            Err(error) => {
                tracing::warn!(error = %without_address(error), "Tautulli cannot be reached");
                None
            }
        };
        match answer.map(|answer| (answer.token, answer.uuid)) {
            Some((Some(token), Some(uuid))) if !token.is_empty() && !uuid.is_empty() => {
                sessions.push(TautulliSession {
                    uuid,
                    token,
                    path: path.clone(),
                });
            }
            _ => tracing::warn!("a Tautulli install gave no session"),
        }
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::{cookie_value, percent_decoded};

    #[test]
    fn cookie_of_an_answer_is_read_and_decoded() {
        let header = "connect.sid=s%3Aabc.def%2Bghi; Path=/; Expires=Wed, 01 Jan 2031 00:00:00 GMT; HttpOnly";

        assert_eq!(
            cookie_value(header, "connect.sid").as_deref(),
            Some("s:abc.def+ghi")
        );
        assert_eq!(cookie_value(header, "other"), None);
        assert_eq!(cookie_value("connect.sid=; Path=/", "connect.sid"), None);
        assert_eq!(cookie_value("no cookie", "connect.sid"), None);
    }

    #[test]
    fn percent_parts_are_decoded() {
        assert_eq!(percent_decoded("a%20b%2"), "a b%2");
        assert_eq!(percent_decoded("plain+text"), "plain+text");
    }
}
