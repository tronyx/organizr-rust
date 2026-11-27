//! The OctoPrint item: the print job of a printer, and its webcam.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::time_limit;
use organizr_integrations::octoprint::{Install, webcam_of};
use serde_json::{Value, json};

use super::{Permissions, is_empty, no_requests, service_url};
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

/// The address of OctoPrint that viewers of the item get. The page needs it only to
/// show a webcam whose stream is a path of the install of OctoPrint. Else it stays empty,
/// since it can name a host of the local network (D-196).
fn address_for_viewers<'a>(url: &'a str, webcam: &Value) -> &'a str {
    let webcam = webcam.get("webcam");
    let on = !is_empty(webcam.and_then(|webcam| webcam.get("webcamEnabled")));
    let at_a_path = webcam
        .and_then(|webcam| webcam.get("streamUrl"))
        .and_then(Value::as_str)
        .is_some_and(|stream| stream.starts_with('/'));
    if on && at_a_path { url } else { "" }
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
            "url": address_for_viewers(&settings.octoprint_url, &webcam),
            "job": job,
            "settings": webcam,
        },
        "options": {
            "title": settings.octoprint_header,
            "titleToggle": settings.octoprint_header_toggle,
        },
    })))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::address_for_viewers;

    #[test]
    fn address_goes_to_viewers_only_for_a_webcam_at_a_path() {
        let url = "http://octopi.local";
        let webcam = |enabled, stream: &str| json!({"webcam": {"webcamEnabled": enabled, "streamUrl": stream}});
        assert_eq!(
            address_for_viewers(url, &webcam(true, "/webcam/?action=stream")),
            url
        );
        assert_eq!(
            address_for_viewers(url, &webcam(true, "http://cam.local/stream")),
            ""
        );
        assert_eq!(
            address_for_viewers(url, &webcam(false, "/webcam/?action=stream")),
            ""
        );
        assert_eq!(address_for_viewers(url, &json!({"webcam": {}})), "");
    }
}
