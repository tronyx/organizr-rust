//! Plex on the homepage: what plays now, what was added, playlists, the details of an
//! item, a search, and the pictures of the server.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Extension;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use organizr_domain::GroupLevel;
use organizr_integrations::http::time_limit;
use organizr_integrations::plex_server::{self, ItemContext, PlexServerError, Server};
use organizr_integrations::tautulli::Install as Tautulli;
use organizr_settings::Settings;
use serde_json::{Map, Value, json};

use super::{Permissions, emby, no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::picture_cache::{self, PictureCache};
use crate::state::AppState;

/// The largest picture of the server that is handed on, in bytes.
pub(super) const PICTURE_LIMIT: usize = 10 * 1024 * 1024;

/// The largest width or height that a picture is asked for, in pixels.
pub(super) const LARGEST_SIDE: u32 = 4000;

/// One of the parts of Plex on the homepage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// What plays now.
    Streams,
    /// What was added.
    Recent,
    /// The playlists.
    Playlists,
    /// The details of one item, and the pictures.
    Metadata,
    /// The search.
    Search,
}

/// What must hold before a caller gets a part.
pub fn permissions(part: Part) -> Permissions {
    let (enabled, auth) = match part {
        Part::Streams => (Some("homepagePlexStreams"), Some("homepagePlexStreamsAuth")),
        Part::Recent => (Some("homepagePlexRecent"), Some("homepagePlexRecentAuth")),
        Part::Playlists => (
            Some("homepagePlexPlaylist"),
            Some("homepagePlexPlaylistAuth"),
        ),
        Part::Metadata => (None, None),
        Part::Search => (Some("mediaSearch"), Some("mediaSearchAuth")),
    };
    let with = |first: &str, more: Option<&str>| {
        std::iter::once(first)
            .chain(more)
            .map(str::to_owned)
            .collect()
    };
    Permissions {
        enabled: with("homepagePlexEnabled", enabled),
        auth: with("homepagePlexAuth", auth),
        not_empty: vec![
            "plexURL".to_owned(),
            "plexToken".to_owned(),
            "plexID".to_owned(),
        ],
    }
}

/// The present time, in seconds since 1970.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_secs()).ok())
        .unwrap_or(0)
}

/// What a request for a part needs: the server, and what the caller sees.
struct Asked {
    settings: std::sync::Arc<Settings>,
    url: String,
    show_names: bool,
    show_addresses: bool,
    now: i64,
    fresh: String,
}

impl Asked {
    fn new(
        state: &AppState,
        caller: &Caller,
        client: &Client,
        headers: &HeaderMap,
        part: Part,
    ) -> Result<Self, Refusal> {
        let settings = state.settings();
        permissions(part).check(&settings, caller)?;
        let may = |level: u32| u32::from(caller.level().get()) <= level;
        let with_address =
            settings.homepage_show_stream_names && may(settings.homepage_show_stream_names_auth);
        let without_address = settings.homepage_show_stream_names_without_ip
            && may(settings.homepage_show_stream_names_without_ip_auth);
        let now = now();
        Ok(Self {
            url: service_url(&settings.plex_url, headers, client),
            show_names: with_address || without_address,
            show_addresses: with_address,
            now,
            // The time makes the address of a picture new, so it is fetched again.
            fresh: format!("{now:x}"),
            settings,
        })
    }

    fn server(&self, refresh: Option<u64>) -> Server<'_> {
        Server {
            url: &self.url,
            token: &self.settings.plex_token,
            timeout: refresh.and_then(|refresh| time_limit(&refresh.to_string())),
            never_check_certificate: self.settings.plex_disable_cert_check,
        }
    }

    fn context<'a>(
        &'a self,
        names: Option<&'a Map<String, Value>>,
        is_cached: &'a (dyn Fn(&str) -> bool + Sync),
    ) -> ItemContext<'a> {
        let scale = self.settings.cache_image_size;
        ItemContext {
            show_names: self.show_names,
            show_addresses: self.show_addresses,
            custom_names: names,
            tab_url: &self.settings.plex_tab_url,
            tab_name: &self.settings.plex_tab_name,
            server_id: &self.settings.plex_id,
            poster: (200 * scale, 300 * scale),
            wide: (1200 * scale, 675 * scale),
            now: self.now,
            fresh: &self.fresh,
            is_cached,
        }
    }

    /// The answer around the items, as the PHP application sends it.
    fn answer(&self, content: Option<Value>) -> Value {
        let mut answer = Map::new();
        if let Some(content) = content {
            answer.insert("content".to_owned(), content);
        }
        answer.insert("plexID".to_owned(), json!(self.settings.plex_id));
        answer.insert("showNames".to_owned(), json!(true));
        answer.insert("group".to_owned(), json!("1"));
        Value::Object(answer)
    }
}

/// Whether the picture cache holds a fresh picture of the server with the given key.
fn cache_check(cache: Option<&PictureCache>) -> impl Fn(&str) -> bool + '_ {
    move |key: &str| cache.is_some_and(|cache| cache.is_fresh(&format!("plex-{key}")))
}

/// The answer when the server sent something that cannot be read, or cannot be
/// reached: `422` with the reason in the data, as in the PHP application.
fn unreadable(error: &PlexServerError) -> Refusal {
    tracing::warn!(%error, "the Plex server gave no usable answer");
    Refusal::RequestWithData(
        StatusCode::UNPROCESSABLE_ENTITY,
        String::new(),
        json!([error.to_string()]),
    )
}

fn excluded(setting: &str) -> Vec<&str> {
    setting.split(',').collect()
}

/// The names that Tautulli has for the users of the Plex server, by their user name.
async fn tautulli_names(
    state: &AppState,
    settings: &Settings,
    headers: &HeaderMap,
    client: &Client,
) -> Map<String, Value> {
    let mut names = Map::new();
    let Some(http) = state.outbound() else {
        return names;
    };
    let address = settings.tautulli_url.split(',').next().unwrap_or_default();
    let url = service_url(address, headers, client);
    let install = Tautulli {
        url: &url,
        key: &settings.tautulli_apikey,
        timeout: None,
        never_check_certificate: settings.tautulli_disable_cert_check,
    };
    let users = match install.command(http, "get_users", &[]).await {
        Ok(Some(response)) => response,
        Ok(None) => return names,
        Err(error) => {
            tracing::warn!(%error, "the names of Tautulli could not be loaded");
            return names;
        }
    };
    for user in users
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let is_local = user.get("user_id").and_then(Value::as_i64) == Some(0);
        let name = user.get("username").and_then(Value::as_str);
        if let (false, Some(name), Some(friendly)) = (is_local, name, user.get("friendly_name")) {
            names.insert(name.to_owned(), friendly.clone());
        }
    }
    names
}

/// The other names of the users, when the settings ask for them: from Tautulli, or
/// else from the setting that holds them.
async fn custom_names(
    state: &AppState,
    settings: &Settings,
    headers: &HeaderMap,
    client: &Client,
) -> Option<Map<String, Value>> {
    if !settings.homepage_use_custom_stream_names {
        return None;
    }
    if !settings.tautulli_url.is_empty() && !settings.tautulli_apikey.is_empty() {
        return Some(tautulli_names(state, settings, headers, client).await);
    }
    serde_json::from_str(&settings.homepage_custom_stream_names).ok()
}

/// `GET /homepage/plex/streams`: what plays on the server now.
pub async fn streams(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Streams)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let names = custom_names(&state, &asked.settings, &headers, &client).await;
    let xml = asked
        .server(Some(asked.settings.homepage_stream_refresh))
        .xml(http, "/status/sessions", &[])
        .await
        .map_err(|error| unreadable(&error))?;
    let Some(xml) = xml else {
        return Err(Refusal::RequestWithData(
            StatusCode::UNAUTHORIZED,
            String::new(),
            json!([]),
        ));
    };
    let is_cached = cache_check(state.picture_cache());
    let items = plex_server::items(
        &xml,
        &asked.context(names.as_ref(), &is_cached),
        &excluded(&asked.settings.homepage_plex_streams_exclude),
        &[],
    )
    .map_err(|error| unreadable(&error))?;
    Ok(Envelope::success(asked.answer(Some(json!(items)))))
}

/// `GET /homepage/plex/recent`: the films, the series and the music that were added
/// last, the newest first.
pub async fn recent(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Recent)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let server = asked.server(Some(asked.settings.homepage_recent_refresh));
    let limit = asked.settings.homepage_recent_limit.to_string();
    let is_cached = cache_check(state.picture_cache());
    let context = asked.context(None, &is_cached);
    let libraries = excluded(&asked.settings.homepage_plex_recent_exclude);

    let mut added: Option<Vec<Value>> = None;
    // Films, series and music.
    for kind in ["1", "2", "8"] {
        let query = [
            ("X-Plex-Container-Start", "0"),
            ("X-Plex-Container-Size", limit.as_str()),
            ("type", kind),
        ];
        let xml = server
            .xml(http, "/hubs/home/recentlyAdded", &query)
            .await
            .map_err(|error| unreadable(&error))?;
        if let Some(xml) = xml {
            let items = plex_server::items(&xml, &context, &libraries, &[])
                .map_err(|error| unreadable(&error))?;
            added.get_or_insert_default().extend(items);
        }
    }
    if let Some(added) = &mut added {
        let time = |item: &Value| item.get("addedAt").and_then(Value::as_i64).unwrap_or(0);
        added.sort_by_key(|item| std::cmp::Reverse(time(item)));
    }
    Ok(Envelope::success(asked.answer(added.map(Value::from))))
}

/// `GET /homepage/plex/playlists`: the video playlists of the server with their
/// videos. A playlist with "private" in its title is left out.
pub async fn playlists(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Playlists)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let server = asked.server(None);
    let xml = server
        .xml(http, "/playlists", &[])
        .await
        .map_err(|error| unreadable(&error))?;
    let Some(xml) = xml else {
        return Err(Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Plex API error".to_owned(),
        ));
    };
    let is_cached = cache_check(state.picture_cache());
    let context = asked.context(None, &is_cached);
    let mut lists = Map::new();
    for playlist in plex_server::playlists(&xml).map_err(|error| unreadable(&error))? {
        // The path comes from the server. It must be a path of the playlists.
        if !playlist.key.starts_with("/playlists/") {
            continue;
        }
        let Some(videos) = server
            .xml(http, &playlist.key, &[])
            .await
            .map_err(|error| unreadable(&error))?
        else {
            continue;
        };
        let videos =
            plex_server::playlist_videos(&videos, &context).map_err(|error| unreadable(&error))?;
        // The PHP application sends the title and the videos in one object, with the
        // place of each video as its key.
        let mut entry = Map::new();
        entry.insert("title".to_owned(), json!(playlist.title));
        for (place, video) in videos.into_iter().enumerate() {
            entry.insert(place.to_string(), video);
        }
        let name: String = playlist
            .title
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect();
        lists.insert(name, Value::Object(entry));
    }
    Ok(Envelope::success(asked.answer(Some(Value::Object(lists)))))
}

/// The items of one path of the server, for the details and the search.
async fn items_at(
    state: &AppState,
    asked: &Asked,
    path: &str,
    query: &[(&str, &str)],
    libraries: &[&str],
    ignored: &[&str],
) -> Result<Envelope<Value>, Refusal> {
    let http = state.outbound().ok_or_else(no_requests)?;
    let xml = asked
        .server(None)
        .xml(http, path, query)
        .await
        .map_err(|error| unreadable(&error))?;
    let Some(xml) = xml else {
        // The PHP application sends its empty answer when the server refuses.
        return Ok(Envelope::success(Value::Null));
    };
    let is_cached = cache_check(state.picture_cache());
    let items = plex_server::items(&xml, &asked.context(None, &is_cached), libraries, ignored)
        .map_err(|error| unreadable(&error))?;
    Ok(Envelope::success(asked.answer(Some(json!(items)))))
}

/// `POST /homepage/plex/metadata`: the details of the item whose key the body names.
pub async fn metadata(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Metadata)?;
    let key = match data.field("key") {
        Some(Value::String(key)) => key.clone(),
        Some(Value::Number(key)) => key.to_string(),
        _ => String::new(),
    };
    // The key of an item is a number. Anything else could name another path.
    if key.is_empty() || !key.chars().all(|digit| digit.is_ascii_digit()) {
        return Err(Refusal::unprocessable("Plex Metadata key is not defined"));
    }
    items_at(
        &state,
        &asked,
        &format!("/library/metadata/{key}"),
        &[],
        &[],
        &[],
    )
    .await
}

/// `GET /homepage/plex/search/{query}`: the items of the server that match a text,
/// without artists and episodes.
pub async fn search(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    Path(query): Path<String>,
) -> Result<Envelope<Value>, Refusal> {
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Search)?;
    if query.is_empty() {
        return Err(Refusal::unprocessable("Plex Metadata key is not defined"));
    }
    let libraries = excluded(&asked.settings.homepage_plex_search_exclude);
    items_at(
        &state,
        &asked,
        "/search",
        &[("query", query.as_str())],
        &libraries,
        &["artist", "episode"],
    )
    .await
}

/// `GET /homepage/tautulli/names`: the names that Tautulli has for the users of the
/// Plex server. For Admins and Co-Admins.
pub async fn names(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    let names = tautulli_names(&state, &settings, &headers, &client).await;
    Ok(Envelope::success(Value::Object(names)))
}

/// `GET /homepage/image`: a picture of the Plex server in a given size. The server
/// keeps the picture for a week. The source `emby` or `jellyfin` asks for a picture of
/// that server instead, see [`emby::image`].
///
/// The query names the path of the picture on the Plex server (`img`), the size
/// (`width`, `height`) and the key of the picture in the cache (`key`). A `$` in the key
/// asks for the picture anew.
pub async fn image(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    uri: axum::http::Uri,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response, Refusal> {
    let not_found = || {
        Refusal::Api(ApiError::NotFound {
            endpoint: uri.path().to_owned(),
            method: "GET".to_owned(),
        })
    };
    let field = |name: &str| query.get(name).map_or_default(String::as_str);
    if let Some(kind) = emby::kind_of(field("source")) {
        let request = emby::Request {
            state: &state,
            caller: &caller,
            client: &client,
            headers: &headers,
        };
        return emby::image(request, kind, &query, &not_found).await;
    }
    let asked = Asked::new(&state, &caller, &client, &headers, Part::Metadata)?;
    if field("source") != "plex" {
        return Err(not_found());
    }
    let (key, refresh) = match field("key").split_once('$') {
        Some((key, _)) => (key, true),
        None => (field("key"), false),
    };
    let name = format!("plex-{key}");
    let cache = state
        .picture_cache()
        .filter(|_| picture_cache::is_cache_name(&name))
        .ok_or_else(not_found)?;
    if !refresh
        && cache.is_fresh(&name)
        && let Some(content) = cache.read(&name).await
    {
        return picture_cache::picture_response(content).ok_or_else(not_found);
    }

    // The picture is a path of the Plex server, and nothing else. The Plex server
    // would fetch any address for the caller.
    let picture = field("img");
    let is_path =
        picture.starts_with('/') && !picture.starts_with("//") && !picture.contains("://");
    let side = |name: &str| {
        field(name)
            .parse::<u32>()
            .ok()
            .filter(|side| (1..=LARGEST_SIDE).contains(side))
    };
    let (true, Some(width), Some(height)) = (is_path, side("width"), side("height")) else {
        return Err(not_found());
    };
    let http = state.outbound().ok_or_else(no_requests)?;
    let content = asked
        .server(None)
        .picture(http, picture, (width, height), PICTURE_LIMIT)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "a picture of the Plex server could not be loaded");
            not_found()
        })?
        .filter(|content| picture_cache::picture_type(content).is_some())
        .ok_or_else(not_found)?;
    if let Err(error) = cache.keep(&name, &content).await {
        tracing::warn!(%error, "a picture of the Plex server could not be kept");
    }
    picture_cache::picture_response(content).ok_or_else(not_found)
}
