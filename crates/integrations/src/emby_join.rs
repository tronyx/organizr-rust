//! A new user on the Emby server of the settings, for a person with an invite: the
//! steps of `embyJoin` of the PHP application.
//!
//! 1. The Admin's key makes the user, without a password.
//! 2. The new user logs in with the empty password, and sets the password.
//! 3. The user gets the policy of the template user that the settings name.
//! 4. The user is linked to the Emby Connect account of the e-mail address. This step
//!    may fail, as in PHP.
//!
//! The Admin's key goes in a header, so it is in no address. No error holds the key,
//! the password or an address.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::Method;
use serde_json::{Value, json};

use crate::emby::is_id;
use crate::http::{Outbound, without_address};

/// How long each step may take.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The fields of the policy of the template user that the new user does not get, as in
/// the PHP application.
const LEFT_OUT: [&str; 3] = [
    "AuthenticationProviderId",
    "InvalidLoginAttemptCount",
    "DisablePremiumFeatures",
];

/// Why the user could not be made.
#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    /// The server cannot be reached. The text names no address.
    #[error("the Emby server cannot be reached: {0}")]
    Unreachable(String),
    /// The server refused a step.
    #[error("the Emby server refused to {0}")]
    Refused(&'static str),
}

/// The Emby server, and the new user.
#[derive(Clone, Copy)]
pub struct Join<'a> {
    /// The address of the server, complete, without a slash at the end.
    pub url: &'a str,
    /// The API key of the Admin.
    pub token: &'a str,
    /// Whether the certificate of the server is left unchecked.
    pub never_check_certificate: bool,
    /// The name of the template user whose policy the new user gets.
    pub template: &'a str,
    /// The name of the new user.
    pub username: &'a str,
    /// The e-mail address of the Emby Connect account.
    pub email: &'a str,
    /// The password of the new user.
    pub password: &'a str,
}

/// The key and the password are secrets, so `Debug` shows neither.
impl std::fmt::Debug for Join<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Join")
            .field("template", &self.template)
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// The device of the requests that the new user sends, as PHP names it: four random
/// digits and a fixed text.
fn device_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    format!(
        "{:04}aWxssS81LgAggFdpbmRvd3MgTlQgMTAuMDsgV2luNjxx7IHf2NCkgQXBwbGVXZWJLaXQvNTM3LjM2IChLSFRNTCwgbGlrZSBHZWNrbykgQ2hyb21lLzcyLjAuMzYyNi4xMTkgU2FmYXJpLzUzNy4zNnwxNTUxNTczMTAyNDI4",
        nanos % 10_000
    )
}

/// The header of a request of the new user.
fn user_header(device: &str, token: Option<&str>) -> String {
    let token = token.map(|token| format!(" Token=\"{}\",", token.replace('"', "")));
    format!(
        "MediaBrowser Client=\"Emby Mobile\", Device=\"Firefox\",{} DeviceId=\"{device}\", Version=\"4.0.2.0\"",
        token.unwrap_or_default()
    )
}

impl Join<'_> {
    fn request(&self, http: &Outbound, method: Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/emby{path}", self.url);
        http.client(&url, self.never_check_certificate)
            .request(method, &url)
            .timeout(TIME_LIMIT)
            .header("Accept", "application/json")
    }

    fn as_admin(&self, http: &Outbound, method: Method, path: &str) -> reqwest::RequestBuilder {
        self.request(http, method, path)
            .header("X-Emby-Token", self.token)
    }

    /// Sends a request, and reads the answer as JSON. A refusal names the step.
    async fn send(
        request: reqwest::RequestBuilder,
        step: &'static str,
    ) -> Result<Value, JoinError> {
        let response = request
            .send()
            .await
            .map_err(|error| JoinError::Unreachable(without_address(error)))?;
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), step, "the Emby server refused a step of a new user");
            return Err(JoinError::Refused(step));
        }
        let text = response
            .text()
            .await
            .map_err(|error| JoinError::Unreachable(without_address(error)))?;
        Ok(serde_json::from_str(&text).unwrap_or(Value::Null))
    }

    /// The policy of the template user, without the fields that a new user does not
    /// get. An empty object when the server has no such user, as in PHP.
    async fn template_policy(&self, http: &Outbound) -> Result<Value, JoinError> {
        let users =
            Self::send(self.as_admin(http, Method::GET, "/Users"), "list the users").await?;
        let template = users.as_array().and_then(|users| {
            users
                .iter()
                .find(|user| user.get("Name").and_then(Value::as_str) == Some(self.template))
        });
        let Some(mut policy) = template
            .and_then(|user| user.get("Policy"))
            .and_then(Value::as_object)
            .cloned()
        else {
            return Ok(json!({}));
        };
        for field in LEFT_OUT {
            policy.remove(field);
        }
        Ok(Value::Object(policy))
    }

    /// The steps after the user exists. A failure leaves the user to be deleted.
    async fn finish(&self, http: &Outbound, id: &str) -> Result<(), JoinError> {
        let device = device_id();
        let login = Self::send(
            self.request(http, Method::POST, "/Users/AuthenticateByName")
                .header("X-Emby-Authorization", user_header(&device, None))
                .json(&json!({"Pw": "", "Username": self.username})),
            "log the new user in",
        )
        .await?;
        let user_token = login
            .get("AccessToken")
            .and_then(Value::as_str)
            .filter(|token| !token.is_empty())
            .ok_or(JoinError::Refused("log the new user in"))?
            .to_owned();
        let header = user_header(&device, Some(&user_token));
        Self::send(
            self.request(http, Method::POST, &format!("/Users/{id}/Password"))
                .header("X-Emby-Authorization", &header)
                .json(&json!({"CurrentPw": "", "NewPw": self.password, "Id": id})),
            "set the password",
        )
        .await?;
        let policy = self.template_policy(http).await?;
        Self::send(
            self.as_admin(http, Method::POST, &format!("/Users/{id}/Policy"))
                .json(&policy),
            "set the policy",
        )
        .await?;
        // The link to Emby Connect is not needed, as in PHP.
        if let Err(error) = Self::send(
            self.request(http, Method::POST, &format!("/Users/{id}/Connect/Link"))
                .header("X-Emby-Authorization", &header)
                .json(&json!({"ConnectUsername": self.email})),
            "link the Emby Connect account",
        )
        .await
        {
            tracing::warn!(%error, "a new Emby user is not linked to Emby Connect");
        }
        Ok(())
    }

    /// Makes the user. When a step after the first fails, the user is deleted again,
    /// so that no user without a password or without the policy stays behind.
    pub async fn run(&self, http: &Outbound) -> Result<(), JoinError> {
        let made = Self::send(
            self.as_admin(http, Method::POST, "/Users/New")
                .query(&[("name", self.username)])
                .json(&json!([])),
            "make the user",
        )
        .await?;
        let id = made
            .get("Id")
            .and_then(Value::as_str)
            .filter(|id| is_id(id))
            .ok_or(JoinError::Refused("make the user"))?
            .to_owned();
        let finished = self.finish(http, &id).await;
        if finished.is_err() {
            let removed = Self::send(
                self.as_admin(http, Method::DELETE, &format!("/Users/{id}")),
                "delete the unfinished user",
            )
            .await;
            if let Err(error) = removed {
                tracing::error!(%error, "an unfinished Emby user could not be deleted");
            }
        }
        finished
    }
}

#[cfg(test)]
mod tests {
    use super::{device_id, user_header};

    #[test]
    fn header_of_the_new_user_has_the_form_of_php() {
        assert_eq!(
            user_header("1234abc", None),
            "MediaBrowser Client=\"Emby Mobile\", Device=\"Firefox\", DeviceId=\"1234abc\", Version=\"4.0.2.0\""
        );
        assert_eq!(
            user_header("1234abc", Some("to\"ken")),
            "MediaBrowser Client=\"Emby Mobile\", Device=\"Firefox\", Token=\"token\", DeviceId=\"1234abc\", Version=\"4.0.2.0\""
        );
        assert!(
            device_id()
                .get(..4)
                .is_some_and(|digits| digits.bytes().all(|b| b.is_ascii_digit()))
        );
    }
}
