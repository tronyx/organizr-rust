//! The UniFi item: the health of the parts of a network, from a UniFi Network
//! controller.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::{Outbound, time_limit};
use organizr_integrations::unifi::{Controller, UnifiError};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageUnifiEnabled".to_owned()],
        auth: vec!["homepageUnifiAuth".to_owned()],
        not_empty: vec![
            "unifiURL".to_owned(),
            "unifiUsername".to_owned(),
            "unifiPassword".to_owned(),
        ],
    }
}

/// What must hold before a caller gets a connection test: all but the switch of the
/// item.
pub fn test_permissions() -> Permissions {
    Permissions {
        enabled: Vec::new(),
        ..permissions()
    }
}

/// The answer for a controller that gave no data, with the status and the text of the
/// PHP application.
pub fn refusal(error: &UnifiError) -> Refusal {
    tracing::warn!(%error, "the UniFi controller gave no data");
    let status = match error {
        UnifiError::Unreachable(_) | UnifiError::NoSites => StatusCode::INTERNAL_SERVER_ERROR,
        UnifiError::Credentials | UnifiError::Refused => StatusCode::CONFLICT,
    };
    Refusal::Request(status, error.to_string())
}

/// The health of the parts of the network, from the controller of the settings.
pub async fn health(settings: &Settings, http: &Outbound) -> Result<Value, UnifiError> {
    let url = service_url(&settings.unifi_url);
    controller(settings, &url).health(http).await
}

/// The controller of the settings, at an address that is complete.
pub fn controller<'a>(settings: &'a Settings, url: &'a str) -> Controller<'a> {
    Controller {
        url,
        username: &settings.unifi_username,
        password: &settings.unifi_password,
        site: &settings.unifi_site_name,
        timeout: time_limit(&settings.homepage_unifi_refresh.to_string()),
        never_check_certificate: settings.unifi_disable_cert_check,
    }
}

/// `GET /homepage/unifi/data`: the health of each part of the network.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.unifi_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let health = health(&settings, http)
        .await
        .map_err(|error| refusal(&error))?;
    Ok(Envelope::success(json!({ "content": { "unifi": health } })))
}
