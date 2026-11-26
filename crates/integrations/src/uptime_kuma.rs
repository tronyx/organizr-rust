//! Uptime Kuma: the monitors of an install, read from its metrics page.
//!
//! Uptime Kuma gives its monitors at `/metrics` in the text format of Prometheus. An
//! API key of Uptime Kuma is the password of the request, with an empty user name.

use std::collections::HashMap;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::http::Outbound;

/// How long Uptime Kuma has to answer.
const TIME_LIMIT: Duration = Duration::from_secs(30);

/// Why the monitors could not be read.
#[derive(Debug, thiserror::Error)]
pub enum KumaError {
    /// Uptime Kuma cannot be reached, or it did not answer in time.
    #[error("Uptime Kuma cannot be reached: {0}")]
    Unreachable(#[source] reqwest::Error),
    /// Uptime Kuma answered with a status that is no success, for example for a wrong
    /// API key.
    #[error("Uptime Kuma answered with the status {0}")]
    Status(u16),
}

/// The text of the metrics page.
pub async fn metrics(http: &Outbound, url: &str, key: &str) -> Result<String, KumaError> {
    let url = format!("{}/metrics", url.trim_end_matches('/'));
    let response = http
        .client(&url, false)
        .get(&url)
        .basic_auth("", Some(key))
        .timeout(TIME_LIMIT)
        .send()
        .await
        .map_err(|error| KumaError::Unreachable(error.without_url()))?;
    if !response.status().is_success() {
        return Err(KumaError::Status(response.status().as_u16()));
    }
    response
        .text()
        .await
        .map_err(|error| KumaError::Unreachable(error.without_url()))
}

/// The text between the first two quotation marks.
fn quoted(text: &str) -> &str {
    text.split('"')
        .nth(1)
        .filter(|_| text.matches('"').count() >= 2)
        .unwrap_or("")
}

/// An address without the user name and the password that it may hold, as in
/// `https://user:password@host/`. The rest of the address stays as it is.
pub fn without_user_info(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    // The host ends where the path, the query or the fragment starts. A backslash ends
    // it too, as for the browser.
    let end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let (authority, after) = rest.split_at(end);
    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://{host}{after}"),
        None => url.to_owned(),
    }
}

/// The monitors of a metrics page: `name`, `url`, `type`, `status` (whether the
/// monitor is up) and `latency` in milliseconds, or `null` when the page has none.
///
/// A monitor whose status is 2 (not checked yet) is left out, as the PHP application
/// leaves it out.
pub fn monitors(page: &str) -> Vec<Value> {
    let lines: Vec<&str> = page
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect();

    // The response time of each monitor, by its name. A later line replaces an
    // earlier one.
    let mut latencies: HashMap<&str, u64> = HashMap::new();
    for line in &lines {
        if !line.starts_with("monitor_response_time") {
            continue;
        }
        let Some(name) = line
            .split_once("monitor_name=\"")
            .and_then(|(_, rest)| rest.rsplit_once("\",monitor_type"))
            .map(|(name, _)| name)
        else {
            continue;
        };
        let Some(value) = line.rsplit(' ').next().and_then(|text| text.parse().ok()) else {
            continue;
        };
        latencies.insert(name, value);
    }

    lines
        .iter()
        .filter(|line| line.starts_with("monitor_status"))
        .filter_map(|line| {
            if line.ends_with('2') {
                return None;
            }
            let labels = line.split_once('{')?.1.split_once('}')?.0;
            let mut monitor = Map::new();
            for label in labels.split(',') {
                let field = if label.starts_with("monitor_name") {
                    "name"
                } else if label.starts_with("monitor_url") {
                    "url"
                } else if label.starts_with("monitor_type") {
                    "type"
                } else {
                    continue;
                };
                let value = quoted(label);
                // The address of a monitor can hold a user name and a password, which
                // the homepage must not show.
                let value = if field == "url" {
                    without_user_info(value)
                } else {
                    value.to_owned()
                };
                monitor.insert(field.to_owned(), Value::from(value));
            }
            monitor.insert("status".to_owned(), Value::Bool(!line.ends_with('0')));
            let name = monitor.get("name").and_then(Value::as_str).unwrap_or("");
            let latency = latencies
                .get(name)
                .map_or(Value::Null, |value| Value::from(*value));
            monitor.insert("latency".to_owned(), latency);
            Some(Value::Object(monitor))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{metrics, monitors, without_user_info};
    use crate::http::Outbound;

    #[test]
    fn url_of_a_monitor_names_no_user_and_no_password() {
        let page = "monitor_status{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://pat:secret@blog.example.com/health\"} 1\n";
        assert_eq!(
            monitors(page),
            [
                json!({"name": "Blog", "type": "http", "url": "https://blog.example.com/health", "status": true, "latency": null})
            ]
        );
        for (url, shown) in [
            (
                "http://token@nas.example.com:8080?x=1",
                "http://nas.example.com:8080?x=1",
            ),
            ("https://a:b@c@host.example/", "https://host.example/"),
            // A backslash ends the host, as for the browser.
            (
                "https://name.example\\x@other/",
                "https://name.example\\x@other/",
            ),
            (
                "https://blog.example.com/a@b",
                "https://blog.example.com/a@b",
            ),
            ("https://blog.example.com", "https://blog.example.com"),
            ("null", "null"),
            ("", ""),
        ] {
            assert_eq!(without_user_info(url), shown, "{url}");
        }
    }

    const PAGE: &str = "# HELP monitor_status Monitor Status (1 = UP, 0= DOWN, 2= PENDING, 3= MAINTENANCE)\n\
# TYPE monitor_status gauge\n\
monitor_status{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 1\n\
monitor_status{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 0\n\
monitor_status{monitor_name=\"New one\",monitor_type=\"http\",monitor_url=\"https://new.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 2\n\
monitor_status{monitor_name=\"Backup\",monitor_type=\"port\",monitor_url=\"null\",monitor_hostname=\"nas.example.com\",monitor_port=\"22\"} 3\n\
# HELP monitor_response_time Monitor Response Time (ms)\n\
# TYPE monitor_response_time gauge\n\
monitor_response_time{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 3\n\
monitor_response_time{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} -1\n\
monitor_cert_days_remaining{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 61\n";

    #[tokio::test]
    async fn error_of_an_install_that_cannot_be_reached_names_no_address() {
        let http = Outbound::new().expect("the client can be built");

        let error = metrics(&http, "http://127.0.0.1:9", "key")
            .await
            .expect_err("nothing answers on port 9");

        let text = error.to_string();
        assert!(text.starts_with("Uptime Kuma cannot be reached"), "{text}");
        assert!(!text.contains("127.0.0.1"), "{text}");
    }

    #[test]
    fn monitors_come_with_status_and_latency() {
        assert_eq!(
            monitors(PAGE),
            [
                json!({"name": "Router", "type": "ping", "url": "null", "status": true, "latency": 3}),
                // A response time that is no whole number from 0 up is no latency.
                json!({"name": "Blog", "type": "http", "url": "https://blog.example.com", "status": false, "latency": null}),
                // A monitor in maintenance counts as up, as in the PHP application.
                json!({"name": "Backup", "type": "port", "url": "null", "status": true, "latency": null}),
            ]
        );
    }

    #[test]
    fn page_of_a_newer_version_with_more_labels_is_read() {
        let page = "monitor_status{monitor_id=\"7\",monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 1\r\n\
monitor_response_time{monitor_id=\"7\",monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 12\r\n";

        assert_eq!(
            monitors(page),
            [
                json!({"name": "Router", "type": "ping", "url": "null", "status": true, "latency": 12})
            ]
        );
    }

    #[test]
    fn text_that_is_no_metrics_page_has_no_monitors() {
        assert_eq!(monitors(""), Vec::<serde_json::Value>::new());
        assert_eq!(
            monitors("<html>Login</html>"),
            Vec::<serde_json::Value>::new()
        );
        assert_eq!(
            monitors("monitor_status 1"),
            Vec::<serde_json::Value>::new()
        );
    }
}
