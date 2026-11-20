//! The Monitorr item: the services that a Monitorr install watches.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::http::time_limit;
use organizr_integrations::monitorr::{self, Install, State as ServiceState};
use serde_json::{Map, Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The picture of a service whose own picture cannot be loaded, as the PHP application
/// names it.
const NO_PICTURE: &str = "plugins/images/homepage/no-list.png";

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageMonitorrEnabled".to_owned()],
        auth: vec!["homepageMonitorrAuth".to_owned()],
        not_empty: vec!["monitorrURL".to_owned()],
    }
}

/// `GET /homepage/monitorr/data`: each service by its name, with its state, its
/// picture and its link. The services that are offline come first.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let url = service_url(&settings.monitorr_url, &headers, &client);
    let install = Install {
        url: &url,
        timeout: time_limit(&settings.homepage_monitorr_refresh.to_string()),
        never_check_certificate: settings.monitorr_disable_cert_check,
    };
    let page = install.page(http).await.map_err(|error| {
        tracing::warn!(%error, "the services of Monitorr could not be read");
        // The PHP application answers 401 for a request that fails.
        Refusal::Request(StatusCode::UNAUTHORIZED, error.to_string())
    })?;

    let mut services: Vec<(usize, monitorr::Service)> =
        monitorr::services(&page).into_iter().enumerate().collect();
    services.sort_by_key(|(place, service)| (service.state != ServiceState::Offline, *place));
    let mut by_name = Map::new();
    for (place, service) in services {
        let picture = match &service.picture {
            Some(path) => install.picture(http, path).await,
            None => None,
        };
        let mut entry = Map::new();
        let status = match service.state {
            ServiceState::Online => json!(true),
            ServiceState::Offline => json!(false),
            ServiceState::Unresponsive => json!("unresponsive"),
        };
        entry.insert("status".to_owned(), status);
        entry.insert("sort".to_owned(), json!(place));
        entry.insert(
            "image".to_owned(),
            json!(picture.unwrap_or_else(|| NO_PICTURE.to_owned())),
        );
        if let Some(link) = service.link {
            entry.insert("link".to_owned(), json!(link));
        }
        by_name.insert(service.name, Value::Object(entry));
    }
    Ok(Envelope::success(json!({
        "services": by_name,
        "options": {
            "title": settings.monitorr_header,
            "titleToggle": settings.monitorr_header_toggle,
            "compact": settings.monitorr_compact,
        },
    })))
}
