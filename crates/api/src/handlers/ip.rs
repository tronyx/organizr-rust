//! `GET /ip/{ip}`: what ipinfo.io knows about an IP address, for an Admin or a Co-Admin.

use std::net::IpAddr;

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_integrations::ipinfo;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::no_requests;
use crate::state::AppState;

/// The message of every failure of the service, as in the PHP application.
const FAILED: &str = "An error occurred";

/// `GET /ip/{ip}`: the answer of ipinfo.io for the address, as the service sends it.
///
/// The path must hold an IP address. The PHP application puts any text into the
/// address of the service.
pub async fn info(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(ip): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let Ok(ip) = ip.parse::<IpAddr>() else {
        return Err(Refusal::unprocessable("No IP Address supplied"));
    };
    let http = state.outbound().ok_or_else(no_requests)?;
    match ipinfo::lookup(http, state.ipinfo(), ip).await {
        Ok(Some(found)) => Ok(Envelope::success(found)),
        Ok(None) => Err(Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            FAILED.to_owned(),
        )),
        Err(reason) => {
            tracing::warn!(reason, "ipinfo.io cannot be reached");
            Err(Refusal::RequestWithData(
                StatusCode::INTERNAL_SERVER_ERROR,
                FAILED.to_owned(),
                Value::from(reason),
            ))
        }
    }
}
