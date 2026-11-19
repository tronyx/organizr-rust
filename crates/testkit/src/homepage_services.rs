//! Stand-ins for the applications whose data the homepage shows.
//!
//! The answers have the shape that the applications give. Every name and value in
//! them is made up.

use axum::Router;
use axum::extract::RawQuery;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};

use crate::services::StandIn;

/// The API keys of the two projects of the Healthchecks stand-in.
pub const HEALTH_KEYS: (&str, &str) = ("made-up-key-of-project-one", "made-up-key-of-project-two");

fn check(name: &str, tags: &str, status: &str) -> Value {
    json!({
        "name": name,
        "slug": name.to_lowercase().replace(' ', "-"),
        "tags": tags,
        "desc": format!("Notes of {name}"),
        "grace": 3600,
        "n_pings": 12,
        "status": status,
        "started": false,
        "last_ping": "2026-11-01T10:00:00+00:00",
        "next_ping": "2026-11-02T10:00:00+00:00",
        "manual_resume": false,
        "methods": "",
        "unique_key": format!("made-up-unique-key-of-{}", name.to_lowercase().replace(' ', "-")),
        "timeout": 86400,
    })
}

/// The checks of the project of a key.
fn checks_of(key: &str) -> Option<Vec<Value>> {
    if key == HEALTH_KEYS.0 {
        Some(vec![
            check("Nightly backup", "prod backup", "up"),
            check("Mail queue", "prod", "down"),
            check("Old job", "", "paused"),
        ])
    } else if key == HEALTH_KEYS.1 {
        Some(vec![
            check("Certificate renewal", "web", "grace"),
            check("Site map", "web prod", "down"),
        ])
    } else {
        None
    }
}

async fn health_checks(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    let key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let Some(checks) = checks_of(key) else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "wrong api key"})),
        )
            .into_response();
    };
    // A check is listed when it has every tag that the query names.
    let wanted: Vec<String> = query
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| pair.strip_prefix("tag=").map(str::to_owned))
        .collect();
    let checks: Vec<Value> = checks
        .into_iter()
        .filter(|check| {
            let tags: Vec<&str> = check["tags"]
                .as_str()
                .unwrap_or_default()
                .split(' ')
                .collect();
            wanted.iter().all(|tag| tags.contains(&tag.as_str()))
        })
        .collect();
    axum::Json(json!({ "checks": checks })).into_response()
}

/// A stand-in for Healthchecks. The address for the settings is its URL with
/// `/api/v1/checks`.
pub async fn healthchecks() -> StandIn {
    StandIn::start(Router::new().route("/api/v1/checks/", get(health_checks))).await
}

/// The API key of the Uptime Kuma stand-in.
pub const KUMA_KEY: &str = "made-up-kuma-key";

/// `Basic` with an empty user name and [`KUMA_KEY`] as the password.
const KUMA_AUTHORIZATION: &str = "Basic Om1hZGUtdXAta3VtYS1rZXk=";

const KUMA_METRICS: &str = "# HELP monitor_status Monitor Status (1 = UP, 0= DOWN, 2= PENDING, 3= MAINTENANCE)\n\
# TYPE monitor_status gauge\n\
monitor_status{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 1\n\
monitor_status{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 0\n\
monitor_status{monitor_name=\"New one\",monitor_type=\"http\",monitor_url=\"https://new.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 2\n\
# HELP monitor_response_time Monitor Response Time (ms)\n\
# TYPE monitor_response_time gauge\n\
monitor_response_time{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 3\n\
monitor_response_time{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} -1\n";

async fn kuma_metrics(headers: HeaderMap) -> Response {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization == Some(KUMA_AUTHORIZATION) {
        (
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            KUMA_METRICS,
        )
            .into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Secure Area\"")],
        )
            .into_response()
    }
}

/// A stand-in for Uptime Kuma, with two monitors that were checked and one that was
/// not checked yet.
pub async fn uptime_kuma() -> StandIn {
    StandIn::start(Router::new().route("/metrics", get(kuma_metrics))).await
}
