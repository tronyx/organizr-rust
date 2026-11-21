//! AdGuard Home: the statistics and the filter lists of one or more installs, for the
//! homepage.

use std::time::Duration;

use base64::Engine;
use serde_json::{Map, Value};

use crate::http::{Outbound, without_address};
use crate::probe::host_of;

/// How long an install has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(10);

/// What the installs answered, each answer by the host of its install.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Statistics {
    /// The answers of `control/stats`.
    pub data: Map<String, Value>,
    /// The answers of `control/filtering/status`.
    pub filters: Map<String, Value>,
}

/// The JSON of a path of an install, or nothing when the install refuses the request
/// or answers with something that is no list and no object.
/// The value of the header `Authorization` for the login of AdGuard Home, or `None`
/// without a user name. PHP sends the user name and the password of the settings to
/// each install as HTTP basic authentication (D-102).
pub fn authorization(username: &str, password: &str) -> Option<String> {
    if username.is_empty() {
        return None;
    }
    let pair = format!("{username}:{password}");
    Some(format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(pair)
    ))
}

async fn read(
    http: &Outbound,
    address: &str,
    path: &str,
    login: Option<&str>,
) -> Result<Option<Value>, String> {
    let url = format!("{}/{path}", address.trim_end_matches('/'));
    let mut request = http.client_for(&url).get(&url).timeout(TIME_LIMIT);
    if let Some(login) = login {
        request = request.header(reqwest::header::AUTHORIZATION, login);
    }
    let response = request.send().await.map_err(without_address)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), path, "AdGuard Home refused the request");
        return Ok(None);
    }
    if let Ok(answer @ (Value::Object(_) | Value::Array(_))) = response.json().await {
        Ok(Some(answer))
    } else {
        tracing::warn!(path, "the answer of AdGuard Home cannot be read");
        Ok(None)
    }
}

/// The statistics and the filter lists of each install. An install that refuses a
/// request is left out of that part.
///
/// Fails when an install cannot be reached. The text of the error names no address.
pub async fn statistics(
    http: &Outbound,
    addresses: &[String],
    login: Option<&str>,
) -> Result<Statistics, String> {
    let mut all = Statistics::default();
    for address in addresses {
        let Some(host) = host_of(address) else {
            continue;
        };
        if let Some(answer) = read(http, address, "control/stats", login).await? {
            all.data.insert(host.clone(), answer);
        }
        if let Some(answer) = read(http, address, "control/filtering/status", login).await? {
            all.filters.insert(host, answer);
        }
    }
    Ok(all)
}
