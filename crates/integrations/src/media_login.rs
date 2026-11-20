//! An Emby server, Emby Connect or a Jellyfin server that checks the password of a
//! login: the back ends `emby_local`, `emby_connect`, `emby_all` and `jellyfin` of the
//! `authBackend` setting.
//!
//! Each function sends what the PHP application sends, and takes the answer for a
//! success where PHP does. A local login of Emby or Jellyfin opens a session at the
//! server, and logs it out again at once; the login counts only when the log-out
//! succeeds, as in PHP. No error holds a password, a key or an address.

use std::time::Duration;

use md5::{Digest as _, Md5};
use serde_json::{Value, json};
use sha1::Sha1;

use crate::emby::EmbyError;
use crate::http::{Outbound, without_address};

/// The address of the API of Emby Connect.
pub const EMBY_CONNECT: &str = "https://connect.emby.media";

/// How long each request may take: the time of the HTTP library of PHP.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// The header with which PHP logs a user in to Emby.
const EMBY_DEVICE: &str = "Emby UserId=\"e8837bc1-ad67-520e-8cd2-f629e3155721\", Client=\"None\", Device=\"Organizr\", DeviceId=\"xxx\", Version=\"1.0.0.0\"";

/// The header with which PHP logs a user in to Jellyfin.
const JELLYFIN_DEVICE: &str = "MediaBrowser Client=\"Organizr Auth\", Device=\"Organizr\", DeviceId=\"orgv2\", Version=\"2.0\"";

/// The account of Emby Connect that logged in, as PHP takes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectAccount {
    /// The name of the account at Emby Connect.
    pub username: String,
    /// The e-mail address of the account at Emby Connect.
    pub email: String,
}

fn unreachable(error: reqwest::Error) -> EmbyError {
    EmbyError::Unreachable(without_address(error))
}

/// Whether a value is true in a loose comparison of PHP (`== true`).
fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(text) => !text.is_empty() && text != "0",
        Value::Array(items) => !items.is_empty(),
        Value::Object(_) => true,
    }
}

/// Whether the answer to a login is a session of a user with a password: it has
/// `SessionInfo` and `User`, and `User.HasPassword` is true. Returns the access token.
fn session_token(answer: &Value) -> Option<String> {
    let has = |key: &str| answer.get(key).is_some_and(|value| !value.is_null());
    if !has("SessionInfo") || !has("User") {
        return None;
    }
    let user = answer.get("User")?;
    if !user.get("HasPassword").is_some_and(is_truthy) {
        return None;
    }
    Some(match answer.get("AccessToken") {
        Some(Value::String(token)) => token.clone(),
        Some(other) if !other.is_null() => other.to_string(),
        _ => String::new(),
    })
}

/// Sends a request and reads the answer as JSON: `None` when the status is no success,
/// and `null` for an answer that is no JSON.
async fn send(request: reqwest::RequestBuilder) -> Result<Option<Value>, EmbyError> {
    let response = request
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(unreachable)?;
    if !response.status().is_success() {
        tracing::debug!(status = %response.status(), "the media server refused a login");
        return Ok(None);
    }
    let text = response.text().await.map_err(unreachable)?;
    Ok(Some(serde_json::from_str(&text).unwrap_or(Value::Null)))
}

/// Sends a request, and says whether the status is a success.
async fn succeeds(request: reqwest::RequestBuilder) -> Result<bool, EmbyError> {
    let response = request
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(unreachable)?;
    Ok(response.status().is_success())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        // Writing into a text cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// Checks a name and a password with the Emby server at `url` (complete, without a
/// slash at the end). `Ok(true)` when the server opens a session of a user who has a
/// password, and closes it again.
pub async fn emby_local(
    http: &Outbound,
    url: &str,
    username: &str,
    password: &str,
) -> Result<bool, EmbyError> {
    let address = format!("{url}/Users/AuthenticateByName");
    // PHP sends the password as it is, and its SHA-1 and MD5 sums, for old servers.
    let body = json!({
        "Username": username,
        "pw": password,
        "Password": hex(&Sha1::digest(password.as_bytes())),
        "PasswordMd5": hex(&Md5::digest(password.as_bytes())),
    });
    let answer = send(
        http.client_for(&address)
            .post(&address)
            .header("Authorization", EMBY_DEVICE)
            .json(&body),
    )
    .await?;
    let Some(token) = answer.as_ref().and_then(session_token) else {
        return Ok(false);
    };
    let address = format!("{url}/Sessions/Logout");
    succeeds(
        http.client_for(&address)
            .post(&address)
            .header("X-Emby-Token", &token)
            .header("X-Mediabrowser-Token", &token)
            .form(&[] as &[(&str, &str)]),
    )
    .await
}

/// Checks a name and a password with the Jellyfin server at `url` (complete, without a
/// slash at the end), as [`emby_local`] does with Emby.
pub async fn jellyfin(
    http: &Outbound,
    url: &str,
    username: &str,
    password: &str,
) -> Result<bool, EmbyError> {
    let address = format!("{url}/Users/authenticatebyname");
    let answer = send(
        http.client_for(&address)
            .post(&address)
            .header("X-Emby-Authorization", JELLYFIN_DEVICE)
            .json(&json!({"Username": username, "Pw": password})),
    )
    .await?;
    let Some(token) = answer.as_ref().and_then(session_token) else {
        return Ok(false);
    };
    tracing::info!("Jellyfin found the user and logged in");
    let address = format!("{url}/Sessions/Logout");
    let header = format!("{JELLYFIN_DEVICE}, Token=\"{}\"", token.replace('"', ""));
    succeeds(
        http.client_for(&address)
            .post(&address)
            .header("X-Emby-Authorization", header)
            .header("Content-Type", "application/json"),
    )
    .await
}

/// Checks a name, or an e-mail address, and a password with Emby Connect at `connect`,
/// and then looks for a user of the Emby server at `url` that is linked to that
/// account. The Admin's key `token` goes in the header `X-Emby-Token`.
pub async fn emby_connect(
    http: &Outbound,
    (connect, url, token): (&str, &str, &str),
    username: &str,
    password: &str,
) -> Result<Option<ConnectAccount>, EmbyError> {
    tracing::info!(user = %username, "a login is tried with Emby Connect");
    let address = format!("{connect}/service/user/authenticate");
    let answer = send(
        http.client_for(&address)
            .post(&address)
            .header("Accept", "application/json")
            .header("X-Application", "Organizr/2.0")
            .form(&[("nameOrEmail", username), ("rawpw", password)]),
    )
    .await?;
    let Some(answer) = answer else {
        tracing::warn!("Emby Connect refused the login");
        return Ok(None);
    };
    let has = |key: &str| answer.get(key).is_some_and(|value| !value.is_null());
    if !has("AccessToken") || !has("User") {
        tracing::warn!("Emby Connect sent an answer that is no login");
        return Ok(None);
    }
    let text = |key: &str| {
        answer
            .get("User")
            .and_then(|user| user.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let account = ConnectAccount {
        username: text("Name"),
        email: text("Email"),
    };

    let address = format!("{url}/Users");
    let Some(users) = send(
        http.client_for(&address)
            .get(&address)
            .header("X-Emby-Token", token),
    )
    .await?
    else {
        return Ok(None);
    };
    let (name, email) = (
        account.username.to_lowercase(),
        account.email.to_lowercase(),
    );
    let linked = users.as_array().into_iter().flatten().any(|user| {
        let link = user
            .get("ConnectLinkType")
            .is_some_and(|value| !value.is_null());
        let connect_name = match user.get("ConnectUserName") {
            Some(Value::String(text)) => text.to_lowercase(),
            Some(other) if !other.is_null() => other.to_string().to_lowercase(),
            _ => return false,
        };
        link && (connect_name == name || connect_name == email)
    });
    if linked {
        tracing::info!(user = %account.username, "Emby has a user that is linked to the account of Emby Connect");
        Ok(Some(account))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{hex, is_truthy, session_token};

    #[test]
    fn only_a_session_of_a_user_with_a_password_counts() {
        let good = json!({"SessionInfo": {}, "User": {"HasPassword": true}, "AccessToken": "t"});
        assert_eq!(session_token(&good), Some("t".to_owned()));
        for bad in [
            json!({"User": {"HasPassword": true}, "AccessToken": "t"}),
            json!({"SessionInfo": {}, "AccessToken": "t"}),
            json!({"SessionInfo": {}, "User": {"HasPassword": false}, "AccessToken": "t"}),
            json!({"SessionInfo": {}, "User": {}, "AccessToken": "t"}),
            json!({"SessionInfo": null, "User": {"HasPassword": true}}),
            json!([]),
            json!(null),
        ] {
            assert_eq!(session_token(&bad), None, "{bad}");
        }
    }

    #[test]
    fn loose_truth_is_that_of_php() {
        for truthy in [
            json!(true),
            json!(1),
            json!("1"),
            json!("yes"),
            json!([0]),
            json!({}),
        ] {
            assert!(is_truthy(&truthy), "{truthy}");
        }
        for falsy in [
            json!(false),
            json!(0),
            json!(""),
            json!("0"),
            json!([]),
            json!(null),
        ] {
            assert!(!is_truthy(&falsy), "{falsy}");
        }
    }

    #[test]
    fn sums_are_written_in_lower_case_hexadecimal() {
        assert_eq!(hex(&[0x00, 0xab, 0x10]), "00ab10");
    }
}
