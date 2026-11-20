//! The route of the health checks plugin: `GET /plugins/healthchecks/run`.

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use organizr_domain::GroupLevel;
use organizr_plugins::health_checks;
use serde_json::Value;

use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::no_requests;
use crate::state::AppState;

/// `GET /plugins/healthchecks/run`: asks every service of the list now, pings
/// Healthchecks with the results, and returns them.
///
/// A scheduled job does the same. This route is for a run by hand, and for a job of
/// another scheduler.
pub async fn run(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    let level = u16::try_from(settings.health_checks_auth_include).unwrap_or(0);
    caller.require(GroupLevel::new(level).unwrap_or(GroupLevel::ADMIN))?;
    // The PHP application gives this answer for a plugin that is off or has no
    // services, too.
    if !health_checks::is_ready(&settings) {
        return Err(Refusal::Request(
            StatusCode::UNAUTHORIZED,
            "User does not have access".to_owned(),
        ));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    Ok(Envelope::success(health_checks::run(http, &settings).await))
}
