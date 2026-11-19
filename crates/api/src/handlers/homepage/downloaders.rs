//! The download clients of the homepage: the queues of SABnzbd and qBittorrent.

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use organizr_domain::GroupLevel;
use organizr_integrations::downloaders::{
    self, Client as Downloader, DownloaderError, TorrentQuery,
};
use organizr_integrations::http::time_limit;
use serde_json::{Map, Value, json};

use super::{Permissions, no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the SABnzbd item.
pub fn sabnzbd_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageSabnzbdEnabled".to_owned()],
        auth: vec!["homepageSabnzbdAuth".to_owned()],
        not_empty: vec!["sabnzbdURL".to_owned(), "sabnzbdToken".to_owned()],
    }
}

/// What must hold before a caller gets the qBittorrent item.
pub fn qbittorrent_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageqBittorrentEnabled".to_owned()],
        auth: vec!["homepageqBittorrentAuth".to_owned()],
        not_empty: vec!["qBittorrentURL".to_owned()],
    }
}

/// The answer when a download client cannot be reached, as in the PHP application.
fn failed(error: &DownloaderError) -> Refusal {
    tracing::warn!(%error, "a download client gave no usable answer");
    match error {
        DownloaderError::Unreachable(_) => {
            Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
        }
        DownloaderError::NoSession => {
            Refusal::conflict("qBittorrent Connect Function - Error: Could not get session ID")
        }
    }
}

/// Asks SABnzbd, after the checks of the item.
async fn ask_sabnzbd(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    fields: &[(&str, &str)],
) -> Result<Option<Value>, Refusal> {
    let settings = state.settings();
    sabnzbd_permissions().check(&settings, caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let url = service_url(&settings.sabnzbd_url, headers, client);
    let install = Downloader {
        url: &url,
        timeout: time_limit(&settings.sabnzbd_refresh.to_string()),
        never_check_certificate: settings.sabnzbd_disable_cert_check,
    };
    downloaders::sabnzbd(http, install, &settings.sabnzbd_token, fields)
        .await
        .map_err(|error| failed(&error))
}

/// `GET /homepage/sabnzbd/queue`: the queue and the history of SABnzbd, as SABnzbd
/// gives them.
pub async fn sabnzbd_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let queue = ask_sabnzbd(&state, &caller, &client, &headers, &[("mode", "queue")]).await?;
    let history = ask_sabnzbd(
        &state,
        &caller,
        &client,
        &headers,
        &[("mode", "history"), ("limit", "100")],
    )
    .await?;
    // A part that SABnzbd did not give is left out, as in the PHP application.
    let mut content = Map::new();
    if let Some(queue) = queue {
        content.insert("queueItems".to_owned(), queue);
    }
    if let Some(history) = history {
        content.insert("historyItems".to_owned(), history);
    }
    let content = if content.is_empty() {
        Value::Bool(false)
    } else {
        Value::Object(content)
    };
    Ok(Envelope::success(json!({ "content": content })))
}

/// Pauses or resumes the queue of SABnzbd, or one download of it. The body names the
/// download in `target`; without it, or with `main`, the whole queue is meant.
async fn switch_sabnzbd(
    state: &AppState,
    caller: &Caller,
    client: &Client,
    headers: &HeaderMap,
    data: &ApiData,
    action: &'static str,
) -> Result<Envelope<Value>, Refusal> {
    // Only an Admin or a Co-Admin stops and starts downloads.
    caller.require(GroupLevel::CO_ADMIN)?;
    let target = match data.field("target") {
        Some(Value::String(target)) => target.clone(),
        Some(Value::Number(target)) => target.to_string(),
        _ => String::new(),
    };
    let fields: Vec<(&str, &str)> = if target.is_empty() || target == "main" {
        vec![("mode", action)]
    } else {
        vec![("mode", "queue"), ("name", action), ("value", &target)]
    };
    let answer = ask_sabnzbd(state, caller, client, headers, &fields).await?;
    Ok(Envelope::success(
        json!({ "content": answer.unwrap_or(Value::Bool(false)) }),
    ))
}

/// `POST /homepage/sabnzbd/queue/pause`.
pub async fn sabnzbd_pause(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    switch_sabnzbd(&state, &caller, &client, &headers, &data, "pause").await
}

/// `POST /homepage/sabnzbd/queue/resume`.
pub async fn sabnzbd_resume(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    switch_sabnzbd(&state, &caller, &client, &headers, &data, "resume").await
}

/// `GET /homepage/qbittorrent/queue`: the torrents of qBittorrent, without the ones
/// that the settings hide.
pub async fn qbittorrent_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    qbittorrent_permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let url = service_url(&settings.qbittorrent_url, &headers, &client);
    let install = Downloader {
        url: &url,
        timeout: time_limit(&settings.qbittorrent_refresh.to_string()),
        never_check_certificate: settings.qbittorrent_disable_cert_check,
    };
    let query = TorrentQuery {
        username: &settings.qbittorrent_username,
        password: &settings.qbittorrent_password,
        first_api: settings.qbittorrent_api_version == 1,
        sort: &settings.qbittorrent_sort_order,
        reverse: settings.qbittorrent_reverse_sorting,
    };
    let torrents = downloaders::qbittorrent(http, install, query)
        .await
        .map_err(|error| failed(&error))?;
    let torrents = downloaders::shown_torrents(
        torrents,
        settings.qbittorrent_hide_seeding,
        settings.qbittorrent_hide_completed,
    );
    Ok(Envelope::success(json!({
        "content": { "queueItems": torrents, "historyItems": false },
    })))
}
