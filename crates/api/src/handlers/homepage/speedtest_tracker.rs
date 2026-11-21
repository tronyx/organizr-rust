//! The speed test item: the last result of a Speedtest Tracker install.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::speedtest_tracker;
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
        enabled: vec!["homepageSpeedtestEnabled".to_owned()],
        auth: vec!["homepageSpeedtestAuth".to_owned()],
        not_empty: vec!["speedtestURL".to_owned()],
    }
}

/// `GET /homepage/speedtest/data`: the last result, with the average and the limits
/// when the install sends them.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.speedtest_use_custom_certificate)
        .ok_or_else(no_requests)?;

    let url = service_url(&settings.speedtest_url, &headers, &client);
    let results = speedtest_tracker::latest(http, &url, settings.speedtest_disable_cert_check)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "Speedtest Tracker cannot be reached");
            Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
        })?
        .ok_or_else(|| Refusal::conflict("SpeedTest connection error"))?;
    Ok(Envelope::success(json!({
        "data": results,
        "options": {
            "title": settings.speedtest_header,
            "titleToggle": settings.speedtest_header_toggle,
        },
    })))
}
