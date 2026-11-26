//! `POST /ping` and `GET /ping/{host}`: whether the applications behind the tabs answer.
//!
//! The PHP application probes every host and port that a request names, for everybody.
//! Here the server probes only an address that a tab of the caller names as its address
//! to check, with the check switched on. Every other entry counts as not answering, and
//! nothing is sent to it (D-08).

use std::collections::{HashMap, HashSet};

use axum::Extension;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use organizr_domain::Tab;
use serde_json::{Map, Value, json};
use tokio::task::JoinSet;

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::handlers::fields::{Refusal, is_nothing_in_php};
use crate::handlers::launch::is_shown;
use crate::ping::{Latency, PingTarget};
use crate::state::AppState;

/// The answer for a request that names nothing to probe, in the words of the PHP
/// application.
fn nothing_to_ping() -> Refusal {
    Refusal::conflict("No ping hostname/IP's entered")
}

/// The addresses that the caller may have probed: the address to check of each tab that
/// the caller sees, when its check is switched on.
async fn allowed(state: &AppState, caller: &Caller) -> Result<Vec<String>, ApiError> {
    let tabs: Vec<Tab> = state.database().tabs().list().await?;
    Ok(tabs
        .into_iter()
        .filter(|tab| tab.ping && is_shown(tab, caller.level()))
        .filter_map(|tab| tab.ping_url)
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty())
        .collect())
}

/// Lets a caller pass who has the level of the `pingAuth` setting, or more access.
fn require_level(state: &AppState, caller: &Caller) -> Result<(), ApiError> {
    if u32::from(caller.level().get()) <= state.settings().ping_auth {
        Ok(())
    } else {
        Err(ApiError::NotAuthorized)
    }
}

/// Probes one entry of a request, when the caller may have it probed. An entry that
/// the caller may not have probed counts as not answering.
async fn latency_of(state: &AppState, allowed: &[String], entry: &str) -> Latency {
    if !allowed.iter().any(|address| address == entry.trim()) {
        return None;
    }
    let target = PingTarget::of(entry)?;
    state.probe(target).await
}

/// Probes every entry, all at the same time, so that one host that does not answer does
/// not hold the others up. The results come in the order of the entries.
///
/// Each address that a tab allows is probed once, however often the entries name it.
/// So one request starts no more probes than the tabs of the caller name, whatever the
/// length of its list.
async fn all_latencies(state: &AppState, allowed: Vec<String>, entries: &[&str]) -> Vec<Latency> {
    let named: HashSet<&str> = entries.iter().map(|entry| entry.trim()).collect();
    let wanted: HashSet<String> = allowed
        .into_iter()
        .filter(|address| named.contains(address.as_str()))
        .collect();
    let mut probes = JoinSet::new();
    for address in wanted {
        let state = state.clone();
        probes.spawn(async move {
            let latency = match PingTarget::of(&address) {
                Some(target) => state.probe(target).await,
                None => None,
            };
            (address, latency)
        });
    }
    let mut results: HashMap<String, Latency> = HashMap::new();
    while let Some(joined) = probes.join_next().await {
        // A probe that broke counts as no answer.
        if let Ok((address, latency)) = joined {
            results.insert(address, latency);
        }
    }
    entries
        .iter()
        .map(|entry| results.get(entry.trim()).copied().flatten())
        .collect()
}

/// `POST /ping`: probes the addresses in the field `list`.
///
/// For a list, the answer holds each entry with its time in milliseconds, or `false` for
/// an entry that did not answer. For one address as text, the answer is its time, or
/// `null`.
pub async fn many(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Response, Refusal> {
    require_level(&state, &caller)?;
    let list = data.field("list").filter(|list| !is_nothing_in_php(list));
    let allowed = allowed(&state, &caller).await?;
    match list {
        None => Err(nothing_to_ping()),
        Some(Value::String(entry)) => {
            let latency = latency_of(&state, &allowed, entry).await;
            Ok(Envelope::success(latency).into_response())
        }
        Some(Value::Array(entries)) => {
            let entries: Vec<&str> = entries
                .iter()
                .map(Value::as_str)
                .collect::<Option<_>>()
                .ok_or_else(|| Refusal::unprocessable("list is not valid"))?;
            let results = all_latencies(&state, allowed, &entries).await;
            let answer: Map<String, Value> = entries
                .iter()
                .zip(results)
                .map(|(entry, latency)| {
                    (
                        (*entry).to_owned(),
                        latency.map_or(json!(false), |time| json!(time)),
                    )
                })
                .collect();
            Ok(Envelope::success(answer).into_response())
        }
        Some(_) => Err(Refusal::unprocessable("list is not valid")),
    }
}

/// `GET /ping/{host}`: probes one address. The answer is its time in milliseconds, or
/// `null` when it did not answer.
pub async fn one(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(host): Path<String>,
    uri: axum::http::Uri,
) -> Result<Response, Refusal> {
    // The router of PHP reads an encoded slash as a slash, and then no route fits.
    if host.contains('/') {
        return Err(ApiError::NotFound {
            endpoint: uri.path().to_owned(),
            method: "GET".to_owned(),
        }
        .into());
    }
    require_level(&state, &caller)?;
    // What PHP counts as "nothing": the text `0`.
    if host == "0" {
        return Err(nothing_to_ping());
    }
    let allowed = allowed(&state, &caller).await?;
    let latency = latency_of(&state, &allowed, &host).await;
    Ok(Envelope::success(latency).into_response())
}
