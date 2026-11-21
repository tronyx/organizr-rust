//! The AdGuard Home item: the statistics of one or more installs.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::adguard;
use serde_json::{Map, Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageAdGuardEnabled".to_owned()],
        auth: vec!["homepageAdGuardAuth".to_owned()],
        not_empty: vec!["adguardURL".to_owned()],
    }
}

/// `GET /homepage/adguard/stats`: the statistics and the filter lists of each install,
/// by its host, and what the item shows of them.
///
/// The requests carry no user name and no password: the PHP application reads them
/// from two settings that it has no default for, and that do not exist here.
pub async fn stats(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let addresses: Vec<String> = settings
        .adguard_url
        .split(',')
        .map(|address| service_url(address, &headers, &client))
        .collect();
    let login = adguard::authorization(&settings.adguard_username, &settings.adguard_password);
    let statistics = adguard::statistics(http, &addresses, login.as_deref())
        .await
        .map_err(|error| {
            tracing::warn!(%error, "AdGuard Home cannot be reached");
            Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
        })?;

    let mut answer = Map::new();
    // A part without an answer is left out, as in the PHP application.
    if !statistics.data.is_empty() {
        answer.insert("data".to_owned(), Value::Object(statistics.data));
    }
    if !statistics.filters.is_empty() {
        answer.insert("filters".to_owned(), Value::Object(statistics.filters));
    }
    answer.insert(
        "options".to_owned(),
        json!({
            "combine": settings.homepage_ad_guard_combine,
            "title": settings.adguard_header_toggle,
            "queries": settings.adguard_queries_toggle,
            "blocked_count": settings.adguard_queries_blocked_toggle,
            "blocked_percent": settings.adguard_percent_toggle,
            "processing_time": settings.adguard_processing_toggle,
            "domain_count": settings.adguard_domain_list_toggle,
        }),
    );
    Ok(Envelope::success(Value::Object(answer)))
}
