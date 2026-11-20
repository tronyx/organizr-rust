//! The API of the PHP project Organizr at `api.organizr.app`. Organizr asks it for the
//! settings of the shared SMTP account that the project offers to its installs.

use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, without_address};

/// The address of the API, as the PHP application has it.
pub const ORGANIZR_API: &str = "https://api.organizr.app";

/// How long the API has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// The answer of the API for the shared SMTP account, as it sends it, or nothing when
/// it refused the request. An answer that is no JSON reads as `null`.
///
/// Fails when the API cannot be reached. The text of the error names no address.
pub async fn shared_smtp(http: &Outbound, base: &str) -> Result<Option<Value>, String> {
    let url = format!("{}/", base.trim_end_matches('/'));
    let response = http
        .client_for(&url)
        .get(&url)
        .query(&[("cmd", "smtp")])
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "the API of Organizr refused the request");
        return Ok(None);
    }
    let text = response.text().await.map_err(without_address)?;
    Ok(Some(serde_json::from_str(&text).unwrap_or(Value::Null)))
}
