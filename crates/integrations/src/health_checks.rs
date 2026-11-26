//! Healthchecks: the checks of one or more projects, for the homepage.
//!
//! The address in the settings is the address of the list of checks, for example
//! `https://healthchecks.io/api/v1/checks`. Each project has an API key of its own.

use std::time::Duration;

use serde_json::Value;

use crate::http::{Outbound, escaped, without_address};

/// The query that asks for checks with the given tags: `?tag=a&tag=b`. Tags come with
/// commas between them, and each is written as one value of the query. No tags, or
/// `*`, ask for every check.
pub fn tags_query(tags: &str) -> String {
    if tags.is_empty() || tags == "0" || tags == "*" {
        String::new()
    } else {
        let fields: Vec<String> = tags
            .split(',')
            .map(|tag| format!("tag={}", escaped(tag)))
            .collect();
        format!("?{}", fields.join("&"))
    }
}

/// The checks of every project whose key is in `keys` (keys with commas between
/// them), sorted by their status.
///
/// A project that does not answer, or answers with an error, is left out.
pub async fn checks(
    http: &Outbound,
    url: &str,
    keys: &str,
    tags: &str,
    timeout: Option<Duration>,
    never_check_certificate: bool,
) -> Vec<Value> {
    let url = format!("{url}/{}", tags_query(tags));
    let mut all = Vec::new();
    for key in keys.split(',') {
        let mut request = http
            .keyed_client(&url, never_check_certificate)
            .get(&url)
            .header("X-Api-Key", key);
        if let Some(timeout) = timeout {
            request = request.timeout(timeout);
        }
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(error = %without_address(error), "Healthchecks cannot be reached");
                continue;
            }
        };
        if !response.status().is_success() {
            tracing::warn!(status = %response.status(), "Healthchecks refused the request");
            continue;
        }
        match response.json::<Value>().await {
            Ok(Value::Object(mut body)) => {
                if let Some(Value::Array(checks)) = body.remove("checks") {
                    all.extend(checks);
                }
            }
            Ok(_) => tracing::warn!("Healthchecks gave no list of checks"),
            Err(error) => tracing::warn!(
                error = %without_address(error),
                "the answer of Healthchecks cannot be read"
            ),
        }
    }
    // Checks with the same status stay in the order of the projects.
    all.sort_by(|a, b| status_of(a).cmp(status_of(b)));
    all
}

fn status_of(check: &Value) -> &str {
    check
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::tags_query;

    #[test]
    fn tags_become_one_query_field_each() {
        assert_eq!(tags_query("prod"), "?tag=prod");
        assert_eq!(tags_query("prod,web"), "?tag=prod&tag=web");
        // A tag is one value of the query, whatever it holds.
        assert_eq!(
            tags_query("a b&key=x,web#top"),
            "?tag=a%20b%26key%3Dx&tag=web%23top"
        );
        for all in ["", "*", "0"] {
            assert_eq!(tags_query(all), "", "{all}");
        }
    }
}
