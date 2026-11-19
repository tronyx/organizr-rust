//! The Uptime Kuma item: the monitors of an Uptime Kuma install.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::uptime_kuma;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageUptimeKumaEnabled".to_owned()],
        auth: vec!["homepageUptimeKumaAuth".to_owned()],
        not_empty: vec!["uptimeKumaURL".to_owned(), "uptimeKumaToken".to_owned()],
    }
}

/// `GET /homepage/kuma/data`: the monitors, and how the item shows them.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let url = service_url(&settings.uptime_kuma_url, &headers, &client);
    let page = uptime_kuma::metrics(http, &url, &settings.uptime_kuma_token)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "the monitors of Uptime Kuma could not be read");
            // The PHP application answers 401 for every failure of this request.
            Refusal::Request(StatusCode::UNAUTHORIZED, error.to_string())
        })?;
    Ok(Envelope::success(json!({
        "data": uptime_kuma::monitors(&page),
        "options": {
            "title": settings.homepage_uptime_kuma_header,
            "titleToggle": settings.homepage_uptime_kuma_header_toggle,
            "compact": settings.homepage_uptime_kuma_compact,
            "showLatency": settings.homepage_uptime_kuma_show_latency,
        },
    })))
}
