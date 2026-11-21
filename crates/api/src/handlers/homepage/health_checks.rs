//! The health checks item: the checks of Healthchecks projects.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use organizr_integrations::health_checks;
use organizr_integrations::http::time_limit;
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
        enabled: vec!["homepageHealthChecksEnabled".to_owned()],
        auth: vec!["homepageHealthChecksAuth".to_owned()],
        not_empty: vec!["healthChecksURL".to_owned(), "healthChecksToken".to_owned()],
    }
}

/// `GET /homepage/healthchecks`: every check.
pub async fn all(
    state: State<AppState>,
    caller: Extension<Caller>,
    client: Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    list(state, caller, client, headers, None).await
}

/// `GET /homepage/healthchecks/{tags}`: the checks with the given tags, which come
/// with commas between them.
pub async fn tagged(
    state: State<AppState>,
    caller: Extension<Caller>,
    client: Extension<Client>,
    headers: HeaderMap,
    Path(tags): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    list(state, caller, client, headers, Some(tags)).await
}

async fn list(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    tags: Option<String>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = &state
        .outbound_for(settings.health_checks_use_custom_certificate)
        .ok_or_else(no_requests)?;

    let url = service_url(&settings.health_checks_url, &headers, &client);
    let checks = health_checks::checks(
        http,
        &url,
        &settings.health_checks_token,
        tags.as_deref().unwrap_or_default(),
        time_limit(&settings.homepage_health_checks_refresh.to_string()),
        settings.health_checks_disable_cert_check,
    )
    .await;
    Ok(Envelope::success(json!({
        "content": { "checks": checks },
        "options": {
            "desc": settings.homepage_health_checks_show_desc,
            "tags": settings.homepage_health_checks_show_tags,
        },
    })))
}
