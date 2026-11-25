//! Netronome: the last speed test of a Netronome install, with the average and the
//! limits of the tests of a time range.
//!
//! Netronome has no API key. The history of its speed tests is public at
//! `api/speedtest/public/history`, so the request carries no login. Every other route of
//! Netronome needs a session, and this application does not ask for one.

use std::time::Duration;

use serde_json::{Map, Number, Value, json};

use crate::http::{Outbound, without_address};

/// How long Netronome has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// The most tests of one answer. A month of tests every hour is fewer.
const MOST_TESTS: u32 = 1000;

/// The time ranges that Netronome knows, with the first one for a text that is none of
/// them.
pub const TIME_RANGES: [&str; 4] = ["week", "24h", "3d", "month"];

/// A time range that Netronome does not know, so it leaves no test out.
const EVERY_TEST: &str = "all";

/// The time range of the settings, or a week for a text that is no time range.
pub fn time_range(setting: &str) -> &'static str {
    TIME_RANGES
        .into_iter()
        .find(|range| *range == setting.trim())
        .unwrap_or(TIME_RANGES[0])
}

/// The results of the speed tests: `current` (the last test, or `null` when the install
/// has none), and `average`, `maximum` and `minimum` of the tests of the time range when
/// the range holds at least two. Nothing when the install refuses the request or answers
/// with something that is no history of Netronome.
///
/// The last test is the last one of the install, also when it is older than the time
/// range.
///
/// Fails when the install cannot be reached. The text of the error names no address.
pub async fn results(
    http: &Outbound,
    url: &str,
    time_range: &str,
    never_check_certificate: bool,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(tests) = history(http, url, time_range, MOST_TESTS, never_check_certificate).await?
    else {
        return Ok(None);
    };
    if !tests.is_empty() {
        return Ok(Some(summary(&tests)));
    }
    // No test in the time range: the last test of the install is an older one.
    let last = history(http, url, EVERY_TEST, 1, never_check_certificate).await?;
    Ok(last.map(|tests| summary(&tests)))
}

/// The tests of a time range, the last one first. Nothing for an answer that is no
/// success or no history.
async fn history(
    http: &Outbound,
    url: &str,
    time_range: &str,
    limit: u32,
    never_check_certificate: bool,
) -> Result<Option<Vec<Value>>, String> {
    let url = format!(
        "{}/api/speedtest/public/history?timeRange={time_range}&page=1&limit={limit}",
        url.trim_end_matches('/')
    );
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
    Ok(answer.get("data").and_then(Value::as_array).cloned())
}

/// The milliseconds of a latency of Netronome. Netronome stores a latency as text: a
/// duration as Go writes it ("12.345ms", "1.2s", "850µs", "1m2s"), or a count of
/// milliseconds without a unit ("12.34").
pub fn milliseconds(latency: &str) -> Option<f64> {
    let text = latency.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<f64>() {
        return number.is_finite().then_some(number);
    }
    let mut total = 0.0;
    let mut rest = text;
    while !rest.is_empty() {
        let digits = rest
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .filter(|end| *end > 0)?;
        let (number, after) = rest.split_at(digits);
        let number: f64 = number.parse().ok()?;
        let letters = after
            .find(|c: char| c.is_ascii_digit() || c == '.')
            .unwrap_or(after.len());
        let (unit, after) = after.split_at(letters);
        total += number
            * match unit {
                "ns" => 0.000_001,
                "us" | "µs" | "μs" => 0.001,
                "ms" => 1.0,
                "s" => 1000.0,
                "m" => 60_000.0,
                "h" => 3_600_000.0,
                _ => return None,
            };
        rest = after;
    }
    Some(total)
}

/// The ping, the download speed and the upload speed of a test. The speeds are megabits
/// in a second.
fn values(test: &Value) -> [Option<f64>; 3] {
    [
        test.get("latency")
            .and_then(Value::as_str)
            .and_then(milliseconds),
        test.get("downloadSpeed").and_then(Value::as_f64),
        test.get("uploadSpeed").and_then(Value::as_f64),
    ]
}

/// A number of an answer, or `null` for no number.
fn number(value: Option<f64>) -> Value {
    value
        .and_then(Number::from_f64)
        .map_or(Value::Null, Value::Number)
}

const FIELDS: [&str; 3] = ["ping", "download", "upload"];

/// One result of the answer from a value for each field.
fn result(values: [Option<f64>; 3]) -> Value {
    Value::Object(
        FIELDS
            .iter()
            .zip(values)
            .map(|(field, value)| ((*field).to_owned(), number(value)))
            .collect(),
    )
}

/// The values of one field of every test that has it.
fn known(all: &[[Option<f64>; 3]], field: usize) -> Vec<f64> {
    all.iter()
        .filter_map(|values| values.get(field).copied().flatten())
        .collect()
}

/// The average of some values, or nothing for no values.
fn average(values: &[f64]) -> Option<f64> {
    let count = u32::try_from(values.len())
        .ok()
        .filter(|count| *count > 0)?;
    Some(values.iter().sum::<f64>() / f64::from(count))
}

/// The results of the tests of an answer, the last test first.
fn summary(tests: &[Value]) -> Map<String, Value> {
    let mut results = Map::new();
    let Some(last) = tests.first() else {
        results.insert("current".to_owned(), Value::Null);
        return results;
    };
    let mut current = result(values(last));
    if let Some(fields) = current.as_object_mut() {
        fields.insert(
            "jitter".to_owned(),
            number(last.get("jitter").and_then(Value::as_f64)),
        );
        for (field, name) in [
            ("serverName", "server"),
            ("testType", "type"),
            ("createdAt", "time"),
        ] {
            let text = last.get(field).and_then(Value::as_str).unwrap_or_default();
            fields.insert(name.to_owned(), json!(text));
        }
    }
    results.insert("current".to_owned(), current);
    if tests.len() < 2 {
        return results;
    }

    let all: Vec<[Option<f64>; 3]> = tests.iter().map(values).collect();
    let each =
        |pick: fn(&[f64]) -> Option<f64>| result([0, 1, 2].map(|field| pick(&known(&all, field))));
    results.insert("average".to_owned(), each(average));
    results.insert(
        "maximum".to_owned(),
        each(|values| values.iter().copied().reduce(f64::max)),
    );
    results.insert(
        "minimum".to_owned(),
        each(|values| values.iter().copied().reduce(f64::min)),
    );
    results.insert("tests".to_owned(), json!(tests.len()));
    results
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{milliseconds, summary, time_range};

    #[test]
    fn latency_is_a_duration_of_go_or_a_count_of_milliseconds() {
        assert_eq!(milliseconds("12.5ms"), Some(12.5));
        assert_eq!(milliseconds("1.2s"), Some(1200.0));
        assert_eq!(milliseconds("850µs"), Some(0.85));
        assert_eq!(milliseconds("850us"), Some(0.85));
        assert_eq!(milliseconds("1m2s"), Some(62_000.0));
        assert_eq!(milliseconds(" 12.34 "), Some(12.34));
        assert_eq!(milliseconds("0s"), Some(0.0));
        for text in ["", "fast", "12 parsecs", "ms", "1.2.3ms", "NaN", "12ms3"] {
            assert_eq!(milliseconds(text), None, "{text}");
        }
    }

    #[test]
    fn time_range_is_one_that_netronome_knows() {
        for range in ["24h", "3d", "week", "month"] {
            assert_eq!(time_range(range), range);
        }
        assert_eq!(time_range(" month "), "month");
        assert_eq!(time_range(""), "week");
        assert_eq!(time_range("all&limit=5"), "week");
    }

    fn test(latency: &str, download: f64, upload: f64) -> Value {
        json!({
            "id": 1,
            "serverName": "A server",
            "testType": "speedtest",
            "downloadSpeed": download,
            "uploadSpeed": upload,
            "latency": latency,
            "jitter": 1.5,
            "createdAt": "2026-11-01T10:00:00Z",
        })
    }

    #[test]
    fn summary_has_the_last_test_and_the_average_and_limits_of_all() {
        let tests = [
            test("12ms", 250.0, 40.0),
            test("20ms", 200.0, 30.0),
            test("no time", 300.0, 35.0),
        ];
        assert_eq!(
            Value::Object(summary(&tests)),
            json!({
                "current": {
                    "ping": 12.0, "download": 250.0, "upload": 40.0, "jitter": 1.5,
                    "server": "A server", "type": "speedtest",
                    "time": "2026-11-01T10:00:00Z",
                },
                "average": {"ping": 16.0, "download": 250.0, "upload": 35.0},
                "maximum": {"ping": 20.0, "download": 300.0, "upload": 40.0},
                "minimum": {"ping": 12.0, "download": 200.0, "upload": 30.0},
                "tests": 3,
            })
        );
    }

    #[test]
    fn summary_of_one_test_has_no_average_and_of_none_has_no_result() {
        let one = summary(&[json!({"latency": "5ms", "downloadSpeed": 100, "uploadSpeed": 20})]);
        assert_eq!(
            Value::Object(one),
            json!({"current": {
                "ping": 5.0, "download": 100.0, "upload": 20.0, "jitter": null,
                "server": "", "type": "", "time": "",
            }})
        );
        assert_eq!(Value::Object(summary(&[])), json!({"current": null}));
    }
}
