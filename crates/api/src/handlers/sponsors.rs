//! `GET /sponsors/opencollective`, `GET /sponsors/github` and `GET /sponsors/all`: the
//! people who give money to the PHP project Organizr, for everybody.
//!
//! The lists come from Open Collective and from the GitHub page of the author of PHP
//! Organizr, at the addresses that the PHP application uses.

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
        SponsorError::Refused => (StatusCode::CONFLICT, refused.to_owned()),
    }
}

async fn open_collective(state: &AppState, http: &Outbound) -> Listed {
    match sponsors::open_collective(http, state.open_collective()).await {
        Ok(Value::Array(list)) => Ok(list),
        Ok(_) => Ok(Vec::new()),
        Err(error) => Err(failure(&error, "Error connecting to Open Collective")),
    }
}

async fn github(state: &AppState, http: &Outbound) -> Listed {
    sponsors::github(http, state.github_sponsors())
        .await
        .map_err(|error| failure(&error, "Error connecting to Github"))
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
