//! `GET /database/journal` and `PUT /database/journal/{option}`: the journal mode of a
//! database on SQLite, as PHP's `getJournalMode()` and `setJournalMode()`.
//!
//! The mode is kept in the database file, as with PHP: a new file gets write-ahead
//! logging (`wal`), and a file keeps the mode that an Admin chose. PostgreSQL has no
//! journal mode, so both routes answer `409` there. `docs/deviations.md` lists the
//! differences as D-97.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use serde::Serialize;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level of the routes: Admins and Co-Admins, as `qualifyRequest(1)`.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The text for a database on PostgreSQL.
pub const ONLY_SQLITE: &str = "The journal mode applies only to a database on SQLite";

/// The answer of PHP: the row of the `PRAGMA`.
#[derive(Debug, Serialize)]
pub struct JournalMode {
    journal_mode: String,
}

/// The answer of a route: the row, or nothing for an option that PHP does not take.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Answer {
    Mode(JournalMode),
    Nothing(NoData),
}

/// `GET /database/journal`.
pub async fn mode(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<JournalMode>, Refusal> {
    caller.require(MANAGER)?;
    if state.database().engine() != organizr_db::Engine::Sqlite {
        return Err(Refusal::conflict(ONLY_SQLITE));
    }
    match state.database().journal_mode().await? {
        Some(mode) if !mode.is_empty() => Ok(Envelope::success(JournalMode { journal_mode: mode })),
        Some(_) => Err(Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error getting Journal Mode".to_owned(),
        )),
        None => Err(Refusal::not_found("Journal Mode not found")),
    }
}

/// `PUT /database/journal/{option}`: `WAL` or `DELETE`, in any letter case. PHP answers
/// another option with `200` and no message, and changes nothing.
pub async fn set_mode(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(option): Path<String>,
) -> Result<Envelope<Answer>, Refusal> {
    caller.require(MANAGER)?;
    let option = option.to_uppercase();
    if !matches!(option.as_str(), "WAL" | "DELETE") {
        return Ok(Envelope::success(Answer::Nothing(NoData)));
    }
    if state.database().engine() != organizr_db::Engine::Sqlite {
        return Err(Refusal::conflict(ONLY_SQLITE));
    }
    match state.database().set_journal_mode(&option).await {
        Ok(Some(mode)) if !mode.is_empty() => {
            tracing::info!(mode, "the journal mode of the database was set");
            Ok(
                Envelope::success(Answer::Mode(JournalMode { journal_mode: mode }))
                    .with_message(format!("Journal Mode updated to: {option}")),
            )
        }
        Ok(Some(_)) => Err(Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error getting Journal Mode".to_owned(),
        )),
        Ok(None) => Err(Refusal::not_found("Journal Mode not found")),
        Err(error) => {
            tracing::warn!(%error, "the journal mode of the database could not be set");
            Err(Refusal::conflict(
                "The journal mode could not be changed while the database is in use. Try again",
            ))
        }
    }
}
