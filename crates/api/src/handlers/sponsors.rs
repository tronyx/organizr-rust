//! `GET /sponsors/opencollective`, `GET /sponsors/github` and `GET /sponsors/all`: the
//! people who give money to the PHP project Organizr, for everybody.
//!
//! The lists come from Open Collective and from the GitHub page of the author of PHP
//! Organizr, at the addresses that the PHP application uses.
//!
//! The routes need no login, so each list is kept for a while: a caller cannot make
//! Organizr ask the two services again and again.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use organizr_integrations::http::Outbound;
use organizr_integrations::sponsors::{self, SponsorError};
use serde_json::Value;

use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::no_requests;
use crate::state::AppState;

/// What one of the two lists gave: the list, or the status and the message of the
/// failure.
type Listed = Result<Vec<Value>, (StatusCode, String)>;

fn failure(error: &SponsorError, refused: &str) -> (StatusCode, String) {
    match error {
        SponsorError::Unreachable(reason) => {
            tracing::warn!(reason, "a list of sponsors could not be fetched");
            (StatusCode::INTERNAL_SERVER_ERROR, reason.clone())
        }
        SponsorError::TooLarge => {
            let reason = error.to_string();
            tracing::warn!(reason, "a list of sponsors could not be fetched");
            (StatusCode::INTERNAL_SERVER_ERROR, reason)
        }
        SponsorError::Refused => (StatusCode::CONFLICT, refused.to_owned()),
    }
}

/// How long a list is kept.
const LIST_KEPT_FOR: Duration = Duration::from_mins(10);

/// How long a failure is kept, so that a service that is down is not asked at once
/// again.
const FAILURE_KEPT_FOR: Duration = Duration::from_mins(1);

/// The lists that were fetched, by their address, with the time of the fetch.
static KEPT: LazyLock<Mutex<HashMap<String, (Instant, Listed)>>> = LazyLock::new(Mutex::default);

/// The list at an address: the kept one while it is fresh, or else a new one.
async fn kept_or(url: &str, fetch: impl Future<Output = Listed>) -> Listed {
    let is_fresh = |(since, listed): &(Instant, Listed)| {
        let kept_for = if listed.is_ok() {
            LIST_KEPT_FOR
        } else {
            FAILURE_KEPT_FOR
        };
        since.elapsed() < kept_for
    };
    if let Some((_, listed)) = KEPT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(url)
        .filter(|kept| is_fresh(kept))
    {
        return listed.clone();
    }
    let listed = fetch.await;
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    kept.retain(|_, entry| is_fresh(entry));
    kept.insert(url.to_owned(), (Instant::now(), listed.clone()));
    listed
}

async fn open_collective(state: &AppState, http: &Outbound) -> Listed {
    let url = state.open_collective();
    kept_or(url, async {
        match sponsors::open_collective(http, url).await {
            Ok(Value::Array(list)) => Ok(list),
            Ok(_) => Ok(Vec::new()),
            Err(error) => Err(failure(&error, "Error connecting to Open Collective")),
        }
    })
    .await
}

async fn github(state: &AppState, http: &Outbound) -> Listed {
    let url = state.github_sponsors();
    kept_or(url, async {
        sponsors::github(http, url)
            .await
            .map_err(|error| failure(&error, "Error connecting to Github"))
    })
    .await
}

/// The `data` of a list: `null` for an empty one, as the PHP application sends it.
fn data_of(list: Vec<Value>) -> Value {
    if list.is_empty() {
        Value::Null
    } else {
        Value::Array(list)
    }
}

fn answer(listed: Listed) -> Result<Envelope<Value>, Refusal> {
    match listed {
        Ok(list) => Ok(Envelope::success(data_of(list))),
        Err((status, message)) => Err(Refusal::Request(status, message)),
    }
}

/// `GET /sponsors/opencollective`: the members at Open Collective.
pub async fn open_collective_list(
    State(state): State<AppState>,
) -> Result<Envelope<Value>, Refusal> {
    let http = state.outbound().ok_or_else(no_requests)?;
    answer(open_collective(&state, http).await)
}

/// `GET /sponsors/github`: the sponsors at GitHub.
pub async fn github_list(State(state): State<AppState>) -> Result<Envelope<Value>, Refusal> {
    let http = state.outbound().ok_or_else(no_requests)?;
    answer(github(&state, http).await)
}

/// `GET /sponsors/all`: both lists in one, ordered by `sortName`. The answer is a
/// success also when a list failed. As in the PHP application, it then carries the
/// message of the last failure.
pub async fn all(State(state): State<AppState>) -> Result<Envelope<Value>, Refusal> {
    let http = state.outbound().ok_or_else(no_requests)?;
    let mut message = None;
    let mut list = Vec::new();
    for listed in [
        open_collective(&state, http).await,
        github(&state, http).await,
    ] {
        match listed {
            // PHP puts the GitHub list in front of the other one, before the sort.
            Ok(mut more) => {
                more.append(&mut list);
                list = more;
            }
            Err((_, failed)) => message = Some(failed),
        }
    }
    list.sort_by_cached_key(|entry| {
        entry
            .get("sortName")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    });
    let envelope = Envelope::success(data_of(list));
    Ok(match message {
        Some(message) => envelope.with_message(message),
        None => envelope,
    })
}
