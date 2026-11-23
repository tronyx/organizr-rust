//! The Plex Libraries plugin of this application (D-121): which libraries of the Plex
//! server each shared account sees, and switches to change them.
//!
//! Admins and Co-Admins see every share and change any library. Another person of the
//! level of `PLEXLIBRARIES-Auth-include` sees the own share, found by the user name or
//! the e-mail address in any letter case, and changes only the libraries of
//! `PLEXLIBRARIES-librariesToInclude`. The other libraries of that share stay as they are.
//! The plugin asks plex.tv with the Plex token and the Plex id of the settings.

use axum::Extension;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use organizr_db::DbError;
use organizr_domain::{GroupLevel, LevelRange, NewTab, TabKind};
use organizr_integrations::plex::{PlexError, SharedServer};
use organizr_settings::Settings;
use serde_json::{Value, json};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The level that sees and changes every share.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// The checks of every route of the plugin: the switch, and the level of the caller.
fn check(state: &AppState, settings: &Settings, caller: &Caller) -> Result<(), Refusal> {
    if !settings.plex_libraries_enabled {
        return Err(Refusal::conflict(
            "PLEXLIBRARIES-enabled module is not enabled",
        ));
    }
    let level = GroupLevel::new(u16::try_from(settings.plex_libraries_auth).unwrap_or(u16::MAX))
        .unwrap_or(GroupLevel::GUEST);
    caller.require(level)?;
    if state.plex().is_none() {
        return Err(Refusal::conflict("Plex cannot be asked"));
    }
    if settings.plex_token.is_empty() || settings.plex_id.is_empty() {
        return Err(Refusal::conflict("Plex Token/ID not set"));
    }
    Ok(())
}

/// Whether a share is the one of the caller: the same user name or e-mail address, in
/// any letter case.
fn is_own(share: &SharedServer, caller: &Caller) -> bool {
    let Some(user) = caller.user() else {
        return false;
    };
    let user = &user.authenticated.user;
    let same = |mine: &str, theirs: &str| !mine.is_empty() && mine.eq_ignore_ascii_case(theirs);
    same(&user.username, &share.username)
        || user
            .email
            .as_deref()
            .is_some_and(|email| same(email, &share.email))
}

/// The ids of the libraries that a person who is no manager may switch.
fn changeable_ids(settings: &Settings) -> Vec<&str> {
    settings
        .plex_libraries_include
        .split([',', ';'])
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect()
}

/// The shares of the server, or the message of the failure.
async fn read_shares(state: &AppState, settings: &Settings) -> Result<Vec<SharedServer>, Refusal> {
    let plex = state
        .plex()
        .ok_or_else(|| Refusal::conflict("Plex cannot be asked"))?;
    plex.shared_servers(&settings.plex_token, &settings.plex_id)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "the shares of the Plex server cannot be read");
            Refusal::conflict(match error {
                PlexError::Status(401) => "Incorrect Token",
                PlexError::Status(404) => "The Plex server was not found",
                _ => "plex.tv cannot be reached",
            })
        })
}

/// `GET /plugins/plexlibraries/shares`: the shares that the caller sees, each with its
/// libraries, whether the share holds each one, and whether the caller may switch it.
pub async fn shares(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    check(&state, &settings, &caller)?;
    let manager = caller.level() <= MANAGER;
    let changeable = changeable_ids(&settings);
    let shares: Vec<Value> = read_shares(&state, &settings)
        .await?
        .into_iter()
        .filter(|share| manager || is_own(share, &caller))
        .map(|share| {
            let libraries: Vec<Value> = share
                .libraries
                .iter()
                .map(|library| {
                    json!({
                        "id": library.id,
                        "title": library.title,
                        "kind": library.kind,
                        "shared": library.shared,
                        "changeable": manager || changeable.contains(&library.id.as_str()),
                    })
                })
                .collect();
            json!({
                "id": share.id,
                "username": share.username,
                "email": share.email,
                "libraries": libraries,
            })
        })
        .collect();
    Ok(Envelope::success(
        json!({ "manager": manager, "shares": shares }),
    ))
}

/// `PUT /plugins/plexlibraries/shares/{share}`: sets the libraries of a share, from the
/// list `libraries` of ids. For a person who is no manager, only the libraries of the
/// setting change, and only in the own share.
pub async fn change(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(share_id): Path<String>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let settings = state.settings();
    check(&state, &settings, &caller)?;
    let manager = caller.level() <= MANAGER;
    let wanted: Vec<String> = match data.field("libraries") {
        Some(Value::Array(ids)) => ids
            .iter()
            .filter_map(|id| match id {
                Value::String(text) => Some(text.clone()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            })
            .collect(),
        _ => return Err(Refusal::unprocessable("libraries was not supplied")),
    };
    let shares = read_shares(&state, &settings).await?;
    let Some(share) = shares.iter().find(|share| share.id == share_id) else {
        return Err(Refusal::Request(
            axum::http::StatusCode::NOT_FOUND,
            "Share not found".to_owned(),
        ));
    };
    if !manager && !is_own(share, &caller) {
        return Err(ApiError::NotAuthorized.into());
    }
    let changeable = changeable_ids(&settings);
    // A library that the caller may switch follows the request; any other keeps its state.
    let libraries: Vec<&str> = share
        .libraries
        .iter()
        .filter(|library| {
            let id = library.id.as_str();
            if manager || changeable.contains(&id) {
                wanted.iter().any(|want| want == id)
            } else {
                library.shared
            }
        })
        .map(|library| library.id.as_str())
        .collect();
    let plex = state
        .plex()
        .ok_or_else(|| Refusal::conflict("Plex cannot be asked"))?;
    plex.update_share(
        &settings.plex_token,
        &settings.plex_id,
        &share.id,
        &libraries,
    )
    .await
    .map_err(|error| {
        tracing::warn!(%error, "a share of the Plex server was not changed");
        Refusal::conflict("plex.tv did not take the change")
    })?;
    tracing::info!(share = %share.id, "the libraries of a share of the Plex server were changed");
    Ok(Envelope::success(NoData).with_message("The libraries are changed"))
}

/// The address of the page of the plugin, as the address of a tab of the kind "Organizr".
pub const PAGE_ADDRESS: &str = "api/v2/plugins/plexlibraries/page";

/// `GET /plugins/plexlibraries/setup/tab`: makes the tab "Plex Libraries" for the page of
/// the plugin, for the level of the plugin, as the bookmark plugin makes its tab.
pub async fn set_up_tab(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Response, Refusal> {
    let settings = state.settings();
    if !settings.plex_libraries_enabled {
        return Err(Refusal::conflict(
            "PLEXLIBRARIES-enabled module is not enabled",
        ));
    }
    caller.require(MANAGER)?;
    let database = state.database();
    if database.tabs().find_by_url(PAGE_ADDRESS).await?.is_some() {
        return Ok(Envelope::success(NoData)
            .with_message("Tab already exists")
            .into_response());
    }
    let failed = || {
        Refusal::Request(
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Tab creation error".to_owned(),
        )
    };
    let category = database
        .categories()
        .find_default()
        .await?
        .ok_or_else(failed)?;
    let level = GroupLevel::new(u16::try_from(settings.plex_libraries_auth).unwrap_or(u16::MAX))
        .unwrap_or(GroupLevel::GUEST);
    let new = NewTab {
        position: database.tabs().next_position().await?,
        category: category.id,
        name: "Plex Libraries".to_owned(),
        url: Some(PAGE_ADDRESS.to_owned()),
        url_local: None,
        is_default: false,
        enabled: true,
        access: LevelRange::new(level, GroupLevel::ADMIN).map_err(|_| failed())?,
        add_to_admin: false,
        image: Some("fontawesome::film".to_owned()),
        kind: TabKind::Internal,
        splash: false,
        ping: false,
        ping_url: None,
        timeout: false,
        timeout_ms: None,
        preload: false,
    };
    match database.tabs().insert(new).await {
        Ok(_) => Ok(Envelope::success(NoData)
            .with_message("Tab created")
            .into_response()),
        Err(DbError::Duplicate) => Err(Refusal::conflict(
            "Tab name: Plex Libraries is already taken",
        )),
        Err(error) => Err(error.into()),
    }
}
