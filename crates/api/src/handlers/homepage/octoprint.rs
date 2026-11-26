//! The OctoPrint item: the print job of a printer, and its webcam.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::time_limit;
use organizr_integrations::octoprint::{Install, webcam_of};
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageOctoprintEnabled".to_owned()],
        auth: vec!["homepageOctoprintAuth".to_owned()],
        not_empty: vec!["octoprintURL".to_owned(), "octoprintToken".to_owned()],
    }
}

/// `GET /homepage/octoprint/data`: the print job, and the webcam of the settings of
/// the printer.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.octoprint_use_custom_certificate)
        .ok_or_else(no_requests)?;

    let url = service_url(&settings.octoprint_url);
    let install = Install {
        url: &url,
        key: &settings.octoprint_token,
        timeout: time_limit(&settings.homepage_octoprint_refresh.to_string()),
        never_check_certificate: settings.octoprint_disable_cert_check,
    };
    let mut answers = Vec::new();
    for endpoint in ["job", "settings"] {
        let answer = install
            .read(http, endpoint)
            .await
            .map_err(|error| {
                tracing::warn!(%error, "OctoPrint cannot be reached");
                Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
            })?
            .ok_or_else(|| Refusal::conflict("OctoPrint connection error"))?;
        answers.push(answer);
    }
    let mut answers = answers.into_iter();
    let job = answers.next().unwrap_or(Value::Null);
    // The settings of OctoPrint hold keys and passwords. Only the webcam is passed on.
    let webcam = webcam_of(&answers.next().unwrap_or(Value::Null));
    Ok(Envelope::success(json!({
        "data": {
            "url": settings.octoprint_url,
            "job": job,
            "settings": webcam,
        },
        "options": {
            "title": settings.octoprint_header,
            "titleToggle": settings.octoprint_header_toggle,
        },
    })))
}
