//! `GET /plex/servers`: the Plex servers of the account whose token the settings hold.
//!
//! An Admin needs the id of the Plex server for the `plexID` setting. This route lists
//! the servers with their ids.

use axum::Extension;
use axum::extract::{RawQuery, State};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_integrations::plex::PlexError;
use serde::Serialize;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::{Refusal, has_key};
use crate::state::AppState;

/// One server, with the names of the PHP application.
#[derive(Debug, Serialize)]
pub struct ServerRow {
    name: String,
    address: String,
    #[serde(rename = "machineIdentifier")]
    machine_identifier: String,
    /// `1` for a server of the account itself, `0` for a server that another account
    /// shared. The PHP application sends a number, not `true` or `false`.
    owned: u8,
}

/// `GET /plex/servers`: every server of the account, or with `owned` in the query only
/// the servers that the account owns.
pub async fn servers(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<Vec<ServerRow>>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    if settings.plex_token.is_empty() {
        return Err(Refusal::unprocessable("Plex Token cannot be empty"));
    }
    let failed = |message: String, data: Value| {
        Refusal::RequestWithData(StatusCode::INTERNAL_SERVER_ERROR, message, data)
    };
    let Some(plex) = state.plex() else {
        return Err(failed("Plex cannot be asked".to_owned(), Value::Null));
    };

    let only_owned = has_key(query.as_deref(), "owned");
    let servers = plex.servers(&settings.plex_token).await.map_err(|error| {
        tracing::warn!(%error, "the Plex servers could not be listed");
        match error {
            // Plex refused the token, or is in trouble.
            PlexError::Status(status) => {
                failed("Plex Error occurred".to_owned(), Value::from(status))
            }
            other => failed(other.to_string(), Value::Null),
        }
    })?;
    Ok(Envelope::success(
        servers
            .into_iter()
            .filter(|server| server.owned || !only_owned)
            .map(|server| ServerRow {
                name: server.name,
                address: server.address,
                machine_identifier: server.machine_identifier,
                owned: u8::from(server.owned),
            })
            .collect(),
    ))
}
