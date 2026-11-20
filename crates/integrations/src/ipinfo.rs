//! ipinfo.io: what the service knows about an IP address, for example its place and
//! its network.

use std::net::IpAddr;
use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, without_address};

/// The address of the service.
pub const IPINFO: &str = "https://ipinfo.io";

/// How long the service has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// What the service knows about the address, as it sends it, or nothing when it refused
/// the request. An answer that is no JSON reads as `null`.
///
/// The request carries no token: the PHP application sends a token of the author of
/// PHP Organizr, which is not used here. Fails when the service cannot be reached. The
/// text of the error names no address.
pub async fn lookup(http: &Outbound, base: &str, ip: IpAddr) -> Result<Option<Value>, String> {
    let url = format!("{}/{ip}/json", base.trim_end_matches('/'));
    let response = http
        .client_for(&url)
        .get(&url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "ipinfo.io refused the request");
        return Ok(None);
    }
    let text = response.text().await.map_err(without_address)?;
    Ok(Some(serde_json::from_str(&text).unwrap_or(Value::Null)))
}
