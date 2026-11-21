//! The Jackett item and the Prowlarr item: a search of the indexers, and the hand-over
//! of a release that was found.
//!
//! The API key of the service never goes to the browser. Both services put their key
//! into the download links of a search, and the key is cut out of every answer here.

use axum::Extension;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use organizr_integrations::indexers::{self, Install, Kind};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::{Refusal, php_text};
use crate::state::AppState;

/// What must hold before a caller gets the item.
///
/// The PHP application reads the address and the key of Prowlarr from `prowlarrURL`
/// and `prowlarrToken`. Its defaults name them `ProwlarrURL` and `ProwlarrToken`, and
/// these are the settings that exist here.
pub fn permissions(kind: Kind) -> Permissions {
    let keys = |enabled: &str, auth: &str, url: &str, token: &str| Permissions {
        enabled: vec![enabled.to_owned()],
        auth: vec![auth.to_owned()],
        not_empty: vec![url.to_owned(), token.to_owned()],
    };
    match kind {
        Kind::Jackett => keys(
            "homepageJackettEnabled",
            "homepageJackettAuth",
            "jackettURL",
            "jackettToken",
        ),
        Kind::Prowlarr => keys(
            "homepageProwlarrEnabled",
            "homepageProwlarrAuth",
            "ProwlarrURL",
            "ProwlarrToken",
        ),
    }
}

/// How the interface shows the item: whether a release goes to the service for the
/// download.
pub fn options(kind: Kind, settings: &Settings) -> Value {
    let black_hole = match kind {
        Kind::Jackett => settings.homepage_jackett_backhole_download,
        Kind::Prowlarr => settings.homepage_prowlarr_backhole_download,
    };
    json!({ "blackHole": black_hole })
}

/// Checks the permissions of the item, or of its connection test, which does not need
/// the switch of the item. A refusal has the text of the PHP application.
pub fn check(
    kind: Kind,
    settings: &Settings,
    caller: &Caller,
    is_test: bool,
) -> Result<(), Refusal> {
    let mut permissions = permissions(kind);
    if is_test {
        permissions.enabled.clear();
    }
    permissions
        .check(settings, caller)
        .map_err(|refusal| match refusal {
            // PHP names the settings of Prowlarr with a small letter at the start.
            Refusal::Request(StatusCode::UNPROCESSABLE_ENTITY, text) if kind == Kind::Prowlarr => {
                Refusal::Request(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    text.replacen("Prowlarr", "prowlarr", 1),
                )
            }
            other => other,
        })
}

/// The address of the install, complete.
pub fn address(kind: Kind, settings: &Settings, headers: &HeaderMap, client: &Client) -> String {
    let stored = match kind {
        Kind::Jackett => &settings.jackett_url,
        Kind::Prowlarr => &settings.prowlarr_url,
    };
    service_url(stored, headers, client)
}

/// Whether the requests to a service also trust the authority that an Admin uploaded.
const fn uses_custom_certificate(kind: Kind, settings: &Settings) -> bool {
    match kind {
        Kind::Jackett => settings.jackett_use_custom_certificate,
        Kind::Prowlarr => settings.prowlarr_use_custom_certificate,
    }
}

/// The install of the settings, at an address that [`address`] made.
pub fn install<'a>(kind: Kind, settings: &'a Settings, url: &'a str) -> Install<'a> {
    let (key, never_check_certificate) = match kind {
        Kind::Jackett => (&settings.jackett_token, settings.jackett_disable_cert_check),
        Kind::Prowlarr => (
            &settings.prowlarr_token,
            settings.prowlarr_disable_cert_check,
        ),
    };
    Install {
        url,
        key,
        never_check_certificate,
    }
}

/// The answer for a service that cannot be reached. The text names no address.
fn unreachable(kind: Kind, error: String) -> Refusal {
    tracing::warn!(%error, service = kind.name(), "the indexers cannot be reached");
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error)
}

/// `false` for no content, as the PHP application sends it.
fn content(found: Option<Value>) -> Value {
    json!({ "content": found.unwrap_or(Value::Bool(false)) })
}

async fn search(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    kind: Kind,
    query: &str,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    check(kind, &settings, caller, false)?;
    if query.is_empty() || query == "0" {
        return Err(Refusal::unprocessable("Query was not supplied"));
    }
    let http = &state
        .outbound_for(uses_custom_certificate(kind, &settings))
        .ok_or_else(no_requests)?;
    let url = address(kind, &settings, headers, client);
    let install = install(kind, &settings, &url);
    let reply = install
        .search(http, kind, query)
        .await
        .map_err(|error| unreachable(kind, error))?;
    // A service that refuses the search gives no content, as in the PHP application.
    let found = reply.is_success.then(|| reply.json(install.key)).flatten();
    Ok(Envelope::success(content(found)))
}

/// `GET /homepage/jackett/{query}`: the releases that the indexers of Jackett have for
/// a text.
pub async fn jackett_search(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(query): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    search(&state, &caller, &client, &headers, Kind::Jackett, &query).await
}

/// `GET /homepage/prowlarr/{query}`: the releases that the indexers of Prowlarr have
/// for a text.
pub async fn prowlarr_search(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(query): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    search(&state, &caller, &client, &headers, Kind::Prowlarr, &query).await
}

/// A field of the body as text, or nothing when PHP takes the field for empty.
fn field(data: &ApiData, name: &str) -> Option<String> {
    data.field(name)
        .and_then(php_text)
        .filter(|text| !text.is_empty() && text != "0")
}

fn failed(message: &str, found: Option<Value>) -> Refusal {
    Refusal::RequestWithData(StatusCode::BAD_REQUEST, message.to_owned(), content(found))
}

/// `POST /homepage/jackett/download/`: Jackett puts the torrent file of a release into
/// its black hole folder. The body names the black hole link of the release, from
/// `/bh/` on.
pub async fn jackett_download(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    let kind = Kind::Jackett;
    let settings = state.settings();
    check(kind, &settings, &caller, false)?;
    let Some(link) = field(&data, "url") else {
        return Err(Refusal::unprocessable("URL was not supplied"));
    };
    let http = &state
        .outbound_for(uses_custom_certificate(kind, &settings))
        .ok_or_else(no_requests)?;
    let url = address(kind, &settings, &headers, &client);
    let install = install(kind, &settings, &url);
    // Only a black hole link of this install is asked for, with the key of the
    // settings.
    let Some(target) = indexers::black_hole_address(&url, &link, install.key) else {
        return Err(Refusal::unprocessable(
            "URL is not a black hole link of Jackett",
        ));
    };
    let reply = install
        .black_hole(http, &target)
        .await
        .map_err(|error| unreachable(kind, error))?;
    let found = reply.is_success.then(|| reply.json(install.key)).flatten();
    let result = found.as_ref().and_then(|answer| answer.get("result"));
    if result.and_then(Value::as_str) == Some("success") {
        return Ok(Envelope::success(content(found)));
    }
    let message = found
        .as_ref()
        .and_then(|answer| answer.get("error"))
        .and_then(Value::as_str)
        .map_or_else(|| "Unknown error".to_owned(), str::to_owned);
    Err(failed(&message, found))
}

/// `POST /homepage/prowlarr/download/`: Prowlarr sends a release of its last search to
/// its download client. The body names the `guid` and the `indexerId` of the release.
pub async fn prowlarr_download(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    let kind = Kind::Prowlarr;
    let settings = state.settings();
    check(kind, &settings, &caller, false)?;
    let Some(guid) = field(&data, "guid") else {
        return Err(Refusal::unprocessable("guid was not supplied"));
    };
    // The id of an indexer is a number.
    let Some(indexer) = field(&data, "indexerId").and_then(|text| text.parse::<i64>().ok()) else {
        return Err(Refusal::unprocessable("indexerId was not supplied"));
    };
    let http = &state
        .outbound_for(uses_custom_certificate(kind, &settings))
        .ok_or_else(no_requests)?;
    let url = address(kind, &settings, &headers, &client);
    let install = install(kind, &settings, &url);
    let reply = install
        .grab(http, &guid, indexer)
        .await
        .map_err(|error| unreachable(kind, error))?;
    if !reply.is_success || reply.body.is_empty() {
        return Err(failed("Unknown error", None));
    }
    // The PHP application sends the answer of Prowlarr as text.
    let text = indexers::text_without_key(&reply.body, install.key);
    Ok(Envelope::success(json!({ "content": text })))
}
