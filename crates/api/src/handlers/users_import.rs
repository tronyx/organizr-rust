//! `POST /users/import/{type}`: creates an account for each user of the Plex, Jellyfin
//! or Emby server of the settings who has none yet, for an Admin or a Co-Admin.
//!
//! Each new account gets a random password that nobody is told, so its user logs in
//! through the media server. Jellyfin and Emby name no e-mail address, so an account
//! from there gets a made-up one that ends in `@placeholder.eml`, as in PHP.

use std::time::SystemTime;

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use organizr_auth::registration::{NewAccount, RegistrationError, create_account};
use organizr_domain::GroupLevel;
use organizr_integrations::emby::{Kind, Server};
use serde_json::Value;
use uuid::Uuid;

use crate::caller::{Caller, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::handlers::fields::Refusal;
use crate::handlers::homepage::service_url;
use crate::state::AppState;

/// A user of a media server: the name and the e-mail address.
type Found = (String, String);

/// A made-up e-mail address, as the PHP application makes one.
fn placeholder_email() -> String {
    let random = Uuid::new_v4().simple().to_string();
    format!("{}@placeholder.eml", random.get(..10).unwrap_or(&random))
}

/// A random password: 32 characters from 122 random bits.
fn random_password() -> String {
    Uuid::new_v4().simple().to_string()
}

/// The friends of the Plex account of the settings, without the managed accounts.
async fn plex_users(state: &AppState) -> Vec<Found> {
    let settings = state.settings();
    if settings.plex_token.is_empty() {
        return Vec::new();
    }
    let Some(plex) = state.plex() else {
        return Vec::new();
    };
    match plex.friends(&settings.plex_token).await {
        Ok(friends) => friends
            .into_iter()
            .filter(|friend| !friend.restricted)
            .map(|friend| (friend.username, friend.email))
            .collect(),
        Err(error) => {
            tracing::error!(%error, "the Plex users could not be listed for an import");
            Vec::new()
        }
    }
}

/// The users of the Jellyfin or the Emby server of the settings.
async fn media_server_users(
    state: &AppState,
    kind: Kind,
    headers: &HeaderMap,
    client: &Client,
) -> Vec<Found> {
    let settings = state.settings();
    let (address, token, never_check) = match kind {
        Kind::Emby => (
            &settings.emby_url,
            &settings.emby_token,
            settings.emby_disable_cert_check,
        ),
        Kind::Jellyfin => (
            &settings.jellyfin_url,
            &settings.jellyfin_token,
            settings.jellyfin_disable_cert_check,
        ),
    };
    if address.is_empty() || token.is_empty() {
        return Vec::new();
    }
    let Some(http) = state.outbound() else {
        return Vec::new();
    };
    let url = service_url(address, headers, client);
    let server = Server {
        kind,
        url: &url,
        token,
        timeout: None,
        never_check_certificate: never_check,
    };
    match server.users(http).await {
        Ok(Some(Value::Array(users))) => users
            .iter()
            .filter_map(|user| user.get("Name").and_then(Value::as_str))
            .map(|name| (name.to_owned(), placeholder_email()))
            .collect(),
        Ok(_) => Vec::new(),
        Err(error) => {
            tracing::error!(%error, server = kind.name(), "the users could not be listed for an import");
            Vec::new()
        }
    }
}

/// `POST /users/import/{type}`: `type` is `plex`, `jellyfin` or `emby`. The answer says
/// how many accounts were created. Another type gets an empty success, as in PHP.
pub async fn import(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(kind): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let found = match kind.as_str() {
        "plex" => plex_users(&state).await,
        "jellyfin" => media_server_users(&state, Kind::Jellyfin, &headers, &client).await,
        "emby" => media_server_users(&state, Kind::Emby, &headers, &client).await,
        _ => return Ok(Envelope::success(NoData)),
    };
    let mut imported = 0_usize;
    for (username, email) in found {
        if username.is_empty() || email.is_empty() {
            continue;
        }
        let account = NewAccount {
            username,
            password: random_password(),
            email,
        };
        match create_account(state.database(), account, now_stamp(SystemTime::now())).await {
            Ok(user) => {
                tracing::info!(user = %user.username, id = %user.id, source = %kind, "a user was imported");
                imported += 1;
            }
            // The user has an account already.
            Err(RegistrationError::Taken { .. }) => {}
            Err(error) => {
                tracing::warn!(%error, "An error occurred during user import");
            }
        }
    }
    Ok(Envelope::success(NoData).with_message(format!("Imported {imported} users")))
}
