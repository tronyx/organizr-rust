//! The routes `/update/*` of PHP Organizr, which update the install by a script, and
//! the new route `GET /update/check`.
//!
//! This application runs from a container image, so an update means pulling the new
//! image (D-23). Each route that would update answers `409` with what the last version
//! check found. `/update/windows` and `/update/linux` give the texts that PHP gives in
//! its container. The migrations of `/update/migrate/{version}` change files and tables
//! of PHP that this application does not have; the import brings the data of an install
//! up to date. `/update/reset/{feature}` does what PHP does. `docs/deviations.md` lists
//! the differences as D-23.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use organizr_domain::GroupLevel;

use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::same_origin::FromThisSite;
use crate::state::AppState;
use crate::update_check::{UpdateStatus, check_for_update};

/// The level of the routes: Admins and Co-Admins, as `qualifyRequest(1)`.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The text of a route that would update the install.
pub const PULL_THE_IMAGE: &str =
    "This install runs from a container image. To update, pull the new image and start it again";

/// The text of `/update/migrate/{version}`.
pub const NO_MIGRATIONS: &str = "The migrations of PHP Organizr do not apply to this install. The import brings the data of an install up to date";

/// The answer `409` with a text and the last result of the version check.
fn refused(state: &AppState, message: &str) -> Envelope<UpdateStatus> {
    Envelope::error(
        StatusCode::CONFLICT,
        message,
        (*state.update_status()).clone(),
    )
}

/// `GET /update`, `/update/docker` and `/update/{download,unzip,move,cleanup}/{branch}`:
/// the updates of PHP by its scripts.
pub async fn by_script(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    caller.require(MANAGER)?;
    Ok(refused(&state, PULL_THE_IMAGE))
}

/// The same for the routes with a branch, which is not used.
pub async fn by_script_for_branch(
    state: State<AppState>,
    caller: Extension<Caller>,
    Path(_branch): Path<String>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    by_script(state, caller).await
}

/// `GET /update/windows`: the text of PHP in its container.
pub async fn windows(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    caller.require(MANAGER)?;
    Ok(refused(&state, "Your install type is not Windows"))
}

/// `GET /update/linux`: the text of PHP in its container.
pub async fn linux(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    caller.require(MANAGER)?;
    Ok(refused(&state, "Your install type is not Linux"))
}

/// `GET /update/migrate/{version}`: nothing to migrate here.
pub async fn migrate(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(_version): Path<String>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    caller.require(MANAGER)?;
    Ok(refused(&state, NO_MIGRATIONS))
}

/// `GET /update/check`: asks GitHub for the newest release now. A request from another
/// site is refused (D-03).
pub async fn check(
    _: FromThisSite,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<UpdateStatus>, ApiError> {
    caller.require(MANAGER)?;
    match check_for_update(&state).await {
        Ok(status) => Ok(Envelope::success(status)),
        Err(_) => Ok(Envelope::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The newest version could not be read from GitHub",
            (*state.update_status()).clone(),
        )),
    }
}

/// `GET /update/reset/{feature}`: sets a feature back to its default, as PHP's
/// `resetUpdateFeature()`. The one feature is the level with the most access that sees
/// a tab, `group_id_max`, which every tab then gives to the Admin. A request from
/// another site is refused (D-03).
pub async fn reset(
    _: FromThisSite,
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(feature): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(MANAGER)?;
    let feature = feature.to_lowercase();
    match feature.as_str() {
        "groupmax" | "groupidmax" | "group-max" | "group-id-max" | "group_id" | "group_id_max" => {
            state.database().tabs().reset_most_access().await?;
        }
        _ => return Err(Refusal::not_found("Feature not found in reset update")),
    }
    tracing::info!(feature, "a feature was reset");
    Ok(Envelope::success(NoData).with_message(format!("Ran reset update feature for {feature}")))
}
