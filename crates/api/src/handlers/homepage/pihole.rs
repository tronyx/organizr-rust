//! The Pi-hole item: the summary of one or more installs.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::pihole;
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
        enabled: vec!["homepagePiholeEnabled".to_owned()],
        auth: vec!["homepagePiholeAuth".to_owned()],
        not_empty: vec!["piholeURL".to_owned()],
    }
}

/// `GET /homepage/pihole/stats`: the summary of each install, by its host.
pub async fn stats(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;

    let installs: Vec<(String, &str)> =
        pihole::installs(&settings.pihole_url, &settings.pihole_token)
            .into_iter()
            .map(|(address, token)| (service_url(address, &headers, &client), token))
            .collect();
    let summaries = pihole::summaries(http, &installs).await.map_err(|error| {
        tracing::warn!(%error, "Pi-hole cannot be reached");
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
    })?;

    let mut answer = Map::new();
    // Without a summary, the answer has no `data`, as in the PHP application.
    if !summaries.is_empty() {
        answer.insert("data".to_owned(), Value::Object(summaries));
    }
    answer.insert(
        "options".to_owned(),
        json!({
            "combine": settings.homepage_pihole_combine,
            "title": settings.pihole_header_toggle,
        }),
    );
    Ok(Envelope::success(Value::Object(answer)))
}
