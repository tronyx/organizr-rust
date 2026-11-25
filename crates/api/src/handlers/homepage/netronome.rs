//! The Netronome item: the last speed test of a Netronome install, with the average and
//! the limits of a time range. PHP Organizr has no such item (D-128).

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::netronome;
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
        enabled: vec!["homepageNetronomeEnabled".to_owned()],
        auth: vec!["homepageNetronomeAuth".to_owned()],
        not_empty: vec!["netronomeURL".to_owned()],
    }
}

/// `GET /homepage/netronome/data`: the last speed test, and the average and the limits
/// of the tests of the time range of the settings.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.netronome_use_custom_certificate)
        .ok_or_else(no_requests)?;

    let url = service_url(&settings.netronome_url, &headers, &client);
    let time_range = netronome::time_range(&settings.homepage_netronome_time_range);
    let results = netronome::results(
        http,
        &url,
        time_range,
        settings.netronome_disable_cert_check,
    )
    .await
    .map_err(|error| {
        tracing::warn!(%error, "Netronome cannot be reached");
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
    })?
    .ok_or_else(|| Refusal::conflict("Netronome connection error"))?;
    Ok(Envelope::success(json!({
        "data": results,
        "options": {
            "title": settings.homepage_netronome_header,
            "titleToggle": settings.homepage_netronome_header_toggle,
            "timeRange": time_range,
        },
    })))
}
