//! AdGuard Home: the statistics and the filter lists of one or more installs, for the
//! homepage.

use std::time::Duration;

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
async fn read(http: &Outbound, address: &str, path: &str) -> Result<Option<Value>, String> {
    let url = format!("{}/{path}", address.trim_end_matches('/'));
    let response = http
        .client_for(&url)
        .get(&url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
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
pub async fn statistics(http: &Outbound, addresses: &[String]) -> Result<Statistics, String> {
    let mut all = Statistics::default();
    for address in addresses {
        let Some(host) = host_of(address) else {
            continue;
        };
        if let Some(answer) = read(http, address, "control/stats").await? {
            all.data.insert(host.clone(), answer);
        }
        if let Some(answer) = read(http, address, "control/filtering/status").await? {
            all.filters.insert(host, answer);
        }
    }
    Ok(all)
}
