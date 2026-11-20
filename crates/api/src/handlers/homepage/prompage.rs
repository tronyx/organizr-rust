//! The PromPage item: the services of a PromPage status page.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::prompage;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item. The item has no level in the PHP
/// application: everybody sees it when it is switched on.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepagePromPageEnabled".to_owned()],
        auth: Vec::new(),
        not_empty: vec!["promPageURL".to_owned()],
    }
}

/// `GET /homepage/prompage/data`: the services, and how the item shows them.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let url = service_url(&settings.prom_page_url, &headers, &client);
    let services = prompage::services(http, &url).await.map_err(|error| {
        tracing::warn!(%error, "the services of PromPage could not be read");
        // The PHP application answers 401 for every failure of this request.
        Refusal::Request(StatusCode::UNAUTHORIZED, error.to_string())
    })?;
    Ok(Envelope::success(json!({
        "data": services,
        "options": {
            "title": settings.homepage_prom_page_header,
            "titleToggle": settings.homepage_prom_page_header_toggle,
            "compact": settings.homepage_prom_page_compact,
            "showUptime": settings.homepage_prom_page_show_uptime,
        },
    })))
}
