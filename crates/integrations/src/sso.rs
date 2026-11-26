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
        .keyed_client_for(&target)
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

/// Asks Komga at `url` for the token of a session of `account`, an e-mail address and a
/// password, and then of the fallback account. Returns `None` when Komga gives none.
///
/// PHP sends the request with curl's `CURLAUTH_ANY`, which answers the Basic
/// authentication that Komga asks for. The header `X-Auth-Token` makes Komga send the
/// token of the session in the same header. Komga keeps it in its cookie `komga_token`.
pub async fn komga_token(
    http: &Outbound,
    url: &str,
    account: Fallback<'_>,
    fallback: Option<Fallback<'_>>,
    timeout: Duration,
) -> Option<String> {
    if let Some(token) = komga_request(http, url, account, timeout).await {
        return Some(token);
    }
    if let Some(fallback) = fallback {
        tracing::warn!("Komga gave no token; trying the fallback account");
        komga_request(http, url, fallback, timeout).await
    } else {
        tracing::warn!("Komga gave no token");
        None
    }
}

async fn komga_request(
    http: &Outbound,
    url: &str,
    account: Fallback<'_>,
    timeout: Duration,
) -> Option<String> {
    let target = format!("{url}/api/v2/users/me");
    let response = http
        .keyed_client_for(&target)
        .get(&target)
        .timeout(timeout)
        .basic_auth(account.user, Some(account.password))
        .header("X-Auth-Token", "organizrSSO")
        .send()
        .await
        .map_err(|error| tracing::warn!(error = %without_address(error), "Komga cannot be reached"))
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    response
        .headers()
        .get("x-auth-token")
        .and_then(|value| value.to_str().ok())
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
}

/// Ends the session of `token` at Komga, at a logout of Organizr, as PHP does. PHP asks
/// `/api/v1/users/logout`, which a current Komga answers with 404 and the session lives
/// on; Komga ends a session at `/api/logout` (D-117).
pub async fn komga_logout(http: &Outbound, url: &str, token: &str, timeout: Duration) {
    let target = format!("{url}/api/logout");
    let sent = http
        .keyed_client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("X-Auth-Token", token)
        .send()
        .await;
    match sent {
        Ok(response) if response.status().is_success() => {
            tracing::info!("the session of Komga was ended");
        }
        Ok(response) => {
            tracing::warn!(status = %response.status(), "Komga did not end its session");
        }
        Err(error) => {
            tracing::warn!(error = %without_address(error), "Komga cannot be reached");
        }
    }
}

#[derive(Deserialize)]
struct PetioAnswer {
    token: Option<String>,
}

/// Asks Petio at `url` for a token: with the Plex token of the login when there is one,
/// else with the name and the password, and then with the fallback account. Returns
/// `None` when Petio gives none. Petio keeps the token in its cookie `petio_jwt`.
pub async fn petio_token(
    http: &Outbound,
    url: &str,
    login: Login<'_>,
    fallback: Option<Fallback<'_>>,
    timeout: Duration,
) -> Option<String> {
    let token = petio_request(http, url, login.username, login.password, login, timeout).await;
    match (token, fallback) {
        (Some(token), _) => Some(token),
        (None, Some(fallback)) => {
            tracing::warn!("Petio gave no token; trying the fallback account");
            let without_plex = Login {
                plex_token: None,
                ..login
            };
            petio_request(
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
            tracing::warn!("Petio gave no token");
            None
        }
    }
}

async fn petio_request(
    http: &Outbound,
    url: &str,
    username: &str,
    password: &str,
    login: Login<'_>,
    timeout: Duration,
) -> Option<String> {
    // The body of PHP: with a Plex token, the name and the password are empty.
    let (path, username, password) = match login.plex_token {
        Some(_) => ("/api/login/plex_login", "", ""),
        None => ("/api/login", username, password),
    };
    let body = json!({
        "user": {"username": username, "password": password, "type": 1},
        "authToken": false,
        "token": login.plex_token,
    });
    let target = format!("{url}{path}");
    let response = http
        .keyed_client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("X-Forwarded-For", login.client_ip)
        .json(&body)
        .send()
        .await
        .map_err(|error| tracing::warn!(error = %without_address(error), "Petio cannot be reached"))
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let answer: PetioAnswer = response.json().await.ok()?;
    answer.token.filter(|token| !token.is_empty())
}

/// The login of the web client of Jellyfin, as the cookies of PHP Organizr hand it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JellyfinSession {
    /// The name under which the web client keeps the user: `user-<user id>-<server id>`.
    pub user_key: String,
    /// The user, as JSON.
    pub user: String,
    /// What the web client keeps as `jellyfin_credentials`: the server, its address in the
    /// browser, the user and the access token, as JSON.
    pub credentials: String,
}

/// The device of the single sign-on to Jellyfin. PHP sends it in `X-Emby-Authorization`,
/// which Jellyfin 12 refuses (D-99), so it goes in `Authorization`.
const JELLYFIN_SSO_DEVICE: &str = r#"MediaBrowser Client="Organizr Jellyfin Tab", Device="Organizr", DeviceId="Organizr_SSO", Version="1.0""#;

#[derive(Deserialize)]
struct JellyfinAnswer {
    #[serde(rename = "AccessToken")]
    access_token: String,
    #[serde(rename = "ServerId")]
    server_id: String,
    #[serde(rename = "User")]
    user: serde_json::Value,
}

/// Asks Jellyfin at `url` for a session of the user with the name and the password of the
/// login. `address` is the address of Jellyfin in the browser, the setting
/// `jellyfinSSOURL`. Returns `None` when Jellyfin gives none.
pub async fn jellyfin_session(
    http: &Outbound,
    url: &str,
    address: &str,
    login: Login<'_>,
    timeout: Duration,
) -> Option<JellyfinSession> {
    let target = format!("{url}/Users/authenticatebyname");
    let response = http
        .keyed_client_for(&target)
        .post(&target)
        .timeout(timeout)
        .header("Authorization", JELLYFIN_SSO_DEVICE)
        .header("Accept", "application/json")
        .header("X-Forwarded-For", login.client_ip)
        .json(&json!({"Username": login.username, "Pw": login.password}))
        .send()
        .await
        .map_err(
            |error| tracing::warn!(error = %without_address(error), "Jellyfin cannot be reached"),
        )
        .ok()?;
    if !response.status().is_success() {
        tracing::warn!("Jellyfin gave no token");
        return None;
    }
    let answer: JellyfinAnswer = response.json().await.ok()?;
    let user_id = answer.user.get("Id")?.as_str()?.to_owned();
    if answer.access_token.is_empty() {
        return None;
    }
    let credentials = json!({"Servers": [{
        "ManualAddress": address,
        "Id": answer.server_id,
        "UserId": user_id,
        "AccessToken": answer.access_token,
    }]});
    Some(JellyfinSession {
        user_key: format!("user-{user_id}-{}", answer.server_id),
        user: answer.user.to_string(),
        credentials: credentials.to_string(),
    })
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
        .keyed_client_for(&target)
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
            .keyed_client_for(&target)
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
