//! Speedtest Tracker: the last result of a Speedtest Tracker install, with the average
//! and the limits when the install sends them.

use std::time::Duration;

use serde_json::{Map, Value};

use crate::http::{Outbound, without_address};

/// How long Speedtest Tracker has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The results of `api/speedtest/latest`: `current`, and `average`, `maximum` and
/// `minimum` when the install sends them. Nothing when the install refuses the
/// request.
///
/// Fails when the install cannot be reached. The text of the error names no address.
pub async fn latest(
    http: &Outbound,
    url: &str,
    never_check_certificate: bool,
) -> Result<Option<Map<String, Value>>, String> {
    let url = format!("{}/api/speedtest/latest", url.trim_end_matches('/'));
    let response = http
        .client(&url, never_check_certificate)
        .get(&url)
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(without_address)?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let answer: Value = response.json().await.unwrap_or(Value::Null);
    Ok(Some(results(&answer)))
}

/// The results of an answer. An old version names the highest values `max`.
fn results(answer: &Value) -> Map<String, Value> {
    let mut results = Map::new();
    results.insert(
        "current".to_owned(),
        answer.get("data").cloned().unwrap_or(Value::Null),
    );
    for (field, name) in [
        ("average", "average"),
        ("max", "maximum"),
        ("maximum", "maximum"),
        ("minimum", "minimum"),
    ] {
        if let Some(value) = answer.get(field) {
            results.insert(name.to_owned(), value.clone());
        }
    }
    results
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::results;

    #[test]
    fn results_have_the_last_one_and_what_else_the_install_sends() {
        let answer = json!({
            "method": "get latest speedtest",
            "data": {"ping": 12.5, "download": 250.1, "upload": 40.2},
            "average": {"ping": 13.0},
            "max": {"ping": 20.0},
            "minimum": {"ping": 9.0},
        });
        assert_eq!(
            Value::Object(results(&answer)),
            json!({
                "current": {"ping": 12.5, "download": 250.1, "upload": 40.2},
                "average": {"ping": 13.0},
                "maximum": {"ping": 20.0},
                "minimum": {"ping": 9.0},
            })
        );
        assert_eq!(
            Value::Object(results(&json!({"data": {"ping": 1}}))),
            json!({"current": {"ping": 1}})
        );
        assert_eq!(
            Value::Object(results(&json!("no object"))),
            json!({"current": null})
        );
    }
}
