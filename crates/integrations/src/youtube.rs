//! YouTube: the search for the trailer of a film or a series, with the API key of the
//! Admin.

use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, without_address};

/// The address of the API of YouTube.
pub const YOUTUBE_API: &str = "https://www.googleapis.com";

/// How long YouTube has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(15);

/// The first short video that YouTube finds for the title and the words "official
/// trailer", as the API sends it. Nothing when YouTube refused the request.
///
/// The key goes in a header, so it is in no address. Fails when YouTube cannot be
/// reached. The text of the error names no address.
pub async fn trailer(
    http: &Outbound,
    api: &str,
    key: &str,
    title: &str,
) -> Result<Option<Value>, String> {
    let url = format!("{}/youtube/v3/search", api.trim_end_matches('/'));
    let wanted = format!("{title} official trailer");
    let response = http
        .client(&url, false)
        .get(&url)
        .query(&[
            ("part", "snippet"),
            ("q", wanted.as_str()),
            ("maxResults", "1"),
            ("type", "video"),
            ("videoDuration", "short"),
        ])
        .header("X-Goog-Api-Key", key)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if !response.status().is_success() {
        tracing::warn!(status = %response.status(), "YouTube refused the search");
        return Ok(None);
    }
    Ok(Some(response.json().await.unwrap_or(Value::Null)))
}
