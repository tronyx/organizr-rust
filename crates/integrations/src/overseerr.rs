//! Overseerr: the requests for films and series, for the homepage.

use std::time::Duration;

use reqwest::{Method, header};
use serde_json::Value;

use crate::http::{Outbound, without_address};

/// How long Overseerr has to answer when the settings name no time.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Overseerr cannot be reached, or it did not answer in time.
#[derive(Debug, thiserror::Error)]
#[error("Overseerr cannot be reached: {0}")]
pub struct OverseerrError(String);

/// What Overseerr answered.
#[derive(Debug, Clone, PartialEq)]
pub struct Reply {
    /// Whether the status is a success.
    pub is_success: bool,
    /// The body as JSON, or `null` when it is none.
    pub body: Value,
}

impl Reply {
    /// The `message` of an answer that is an error, when it has one.
    pub fn message(&self) -> Option<&str> {
        self.body.get("message").and_then(Value::as_str)
    }
}

/// An install of Overseerr, and how it is asked.
#[derive(Debug, Clone)]
pub struct Install {
    /// The address, complete.
    pub url: String,
    /// The API key.
    pub key: String,
    /// How long the install has to answer.
    pub timeout: Option<Duration>,
    /// Whether the certificate of the install is left unchecked.
    pub never_check_certificate: bool,
}

/// Who a request to Overseerr comes from.
#[derive(Debug, Clone, Copy)]
pub enum As<'a> {
    /// The install of Organizr, with its API key.
    Application,
    /// A user, with the value of the `connect.sid` cookie that single sign-on gave the
    /// browser.
    User(&'a str),
}

impl Install {
    /// Sends a request to a path under `/api/v1`, with a JSON body when one is given.
    pub async fn send(
        &self,
        http: &Outbound,
        method: Method,
        path: &str,
        body: Option<&Value>,
        who: As<'_>,
    ) -> Result<Reply, OverseerrError> {
        let url = format!("{}/api/v1/{path}", self.url.trim_end_matches('/'));
        let mut request = http
            .client(&url, self.never_check_certificate)
            .request(method, &url)
            .header(header::ACCEPT, "application/json")
            .timeout(self.timeout.unwrap_or(TIME_LIMIT));
        request = match who {
            As::Application => request.header("X-Api-Key", &self.key),
            As::User(session) => request.header(header::COOKIE, format!("connect.sid={session}")),
        };
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request
            .send()
            .await
            .map_err(|error| OverseerrError(without_address(error)))?;
        let is_success = response.status().is_success();
        let body = response.json().await.unwrap_or(Value::Null);
        Ok(Reply { is_success, body })
    }

    /// Reads a path under `/api/v1` with the API key.
    pub async fn get(&self, http: &Outbound, path: &str) -> Result<Reply, OverseerrError> {
        self.send(http, Method::GET, path, None, As::Application)
            .await
    }
}

/// The Sonarr or Radarr of Overseerr that a new request goes to: the default one, in
/// 4K or not as the setting says, or else the first one.
pub fn default_service(services: &Value, prefer_4k: bool) -> Option<&Value> {
    let services = services.as_array()?;
    let is_true =
        |service: &Value, field: &str| service.get(field).and_then(Value::as_bool).unwrap_or(false);
    // The PHP application takes the last default that fits, and never the one at the
    // first place of the list by this rule: that one is the fallback anyway.
    services
        .iter()
        .skip(1)
        .rfind(|service| is_true(service, "isDefault") && is_true(service, "is4k") == prefer_4k)
        .or_else(|| services.first())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::default_service;

    #[test]
    fn default_service_is_the_default_of_the_wanted_quality_or_the_first() {
        let services = json!([
            {"id": 1, "isDefault": false, "is4k": false},
            {"id": 2, "isDefault": true, "is4k": false},
            {"id": 3, "isDefault": true, "is4k": true},
        ]);
        let id =
            |prefer_4k| default_service(&services, prefer_4k).map(|service| service["id"].clone());

        assert_eq!(id(false), Some(json!(2)));
        assert_eq!(id(true), Some(json!(3)));
        // No default of the wanted quality: the first service.
        let plain = json!([{"id": 1, "isDefault": true, "is4k": false}]);
        assert_eq!(
            default_service(&plain, true).map(|service| service["id"].clone()),
            Some(json!(1))
        );
        assert_eq!(default_service(&json!([]), false), None);
        assert_eq!(default_service(&json!({"message": "no"}), false), None);
    }
}
