//! `GET /trusted-hosts`: the allowed hosts and the trusted proxies, with where each list
//! comes from: a variable of the container, which wins, or the setting (D-155).

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use organizr_domain::GroupLevel;
use serde::Serialize;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// One list as the settings page shows it.
#[derive(Debug, Serialize)]
pub struct List {
    /// The entries that count, with commas between them.
    value: String,
    /// The variable of the container that sets the list when it names any entry.
    variable: &'static str,
    /// Whether the variable sets the list. The setting then does not count.
    from_variable: bool,
}

/// Both lists.
#[derive(Debug, Serialize)]
pub struct Lists {
    #[serde(rename = "allowedHosts")]
    allowed_hosts: List,
    #[serde(rename = "trustedProxies")]
    trusted_proxies: List,
}

/// `GET /trusted-hosts`: for an Admin or a Co-Admin.
pub async fn lists(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Lists>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let hosts = state.allowed_hosts_of_the_variable();
    let proxies = state.trusted_proxies_of_the_variable();
    Ok(Envelope::success(Lists {
        allowed_hosts: List {
            value: state.allowed_hosts().text(),
            variable: "ORGANIZR_ALLOWED_HOSTS",
            from_variable: !hosts.is_empty(),
        },
        trusted_proxies: List {
            value: state.trusted_proxies().text(),
            variable: "ORGANIZR_TRUSTED_PROXIES",
            from_variable: !proxies.is_empty(),
        },
    }))
}

/// Stores the host of this request as the allowed host of the setting, when neither the
/// variable nor the setting names one. Only the set-up and the login of an Admin call
/// it, so the host is one that the Admin used, as after an import (D-155).
pub async fn remember_host(state: &AppState, headers: &HeaderMap) {
    if !state.allowed_hosts().is_empty() {
        return;
    }
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .filter(|host| {
            host.parse::<crate::allowed_hosts::AllowedHosts>()
                .is_ok_and(|list| !list.is_empty())
        });
    let Some(host) = host else {
        return;
    };
    let change = vec![("allowedHosts".to_owned(), Value::String(host.to_owned()))];
    match state.change_settings(change).await {
        Ok(()) => tracing::info!(host, "the host of Organizr was stored as an allowed host"),
        Err(error) => tracing::warn!(%error, "the host of Organizr could not be stored"),
    }
}
