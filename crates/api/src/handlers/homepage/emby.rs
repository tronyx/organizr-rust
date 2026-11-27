//! Emby and Jellyfin on the homepage: what plays now, what was added, the details of
//! an item, and the pictures of the server.
//!
//! The two servers answer in the same way, so one set of handlers serves both. Each
//! server has settings of its own.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use organizr_integrations::emby::{self, EmbyError, ItemContext, Kind, Server};
use organizr_integrations::http::{Outbound, time_limit};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::plex::{LARGEST_SIDE, PICTURE_LIMIT};
use super::{Permissions, no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::picture_cache::{self, PictureCache};
use crate::state::AppState;

/// How long the server has to answer the question for the details of an item.
const DETAILS_TIME_LIMIT: Duration = Duration::from_secs(60);

/// The types of pictures that the homepage asks for.
const PICTURE_TYPES: [&str; 3] = ["Primary", "Thumb", "Backdrop"];

/// One of the parts of a server on the homepage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// What plays now.
    Streams,
    /// What was added.
    Recent,
    /// The details of one item, the pictures, and the connection test of Emby.
    Metadata,
}

/// What must hold before a caller gets a part.
pub fn permissions(kind: Kind, part: Part) -> Permissions {
    let (enabled, auth) = match (kind, part) {
        (Kind::Emby, Part::Streams) => {
            (Some("homepageEmbyStreams"), Some("homepageEmbyStreamsAuth"))
        }
        (Kind::Emby, Part::Recent) => (Some("homepageEmbyRecent"), Some("homepageEmbyRecentAuth")),
        // The key of this level has the short name in the PHP application.
        (Kind::Jellyfin, Part::Streams) => (
            Some("homepageJellyfinStreams"),
            Some("homepageJellyStreamsAuth"),
        ),
        (Kind::Jellyfin, Part::Recent) => (
            Some("homepageJellyfinRecent"),
            Some("homepageJellyfinRecentAuth"),
        ),
        (_, Part::Metadata) => (None, None),
    };
    let with = |first: &str, more: Option<&str>| {
        std::iter::once(first)
            .chain(more)
            .map(str::to_owned)
            .collect()
    };
    let (switch, level, address, token) = match kind {
        Kind::Emby => (
            "homepageEmbyEnabled",
            "homepageEmbyAuth",
            "embyURL",
            "embyToken",
        ),
        Kind::Jellyfin => (
            "homepageJellyfinEnabled",
            "homepageJellyfinAuth",
            "jellyfinURL",
            "jellyfinToken",
        ),
    };
    Permissions {
        enabled: with(switch, enabled),
        auth: with(level, auth),
        not_empty: vec![address.to_owned(), token.to_owned()],
    }
}

/// The settings of one of the two servers.
pub(super) struct Install<'a> {
    pub(super) address: &'a str,
    pub(super) token: &'a str,
    pub(super) never_check: bool,
    /// Whether the requests also trust the authority that an Admin uploaded.
    pub(super) custom_certificate: bool,
    pub(super) link: &'a str,
    pub(super) open_tab: bool,
    pub(super) tab_name: &'a str,
}

pub(super) fn install(settings: &Settings, kind: Kind) -> Install<'_> {
    match kind {
        Kind::Emby => Install {
            address: &settings.emby_url,
            token: &settings.emby_token,
            never_check: settings.emby_disable_cert_check,
            custom_certificate: settings.emby_use_custom_certificate,
            link: &settings.homepage_emby_link,
            open_tab: !settings.emby_tab_url.is_empty() && !settings.emby_tab_name.is_empty(),
            tab_name: &settings.emby_tab_name,
        },
        Kind::Jellyfin => Install {
            address: &settings.jellyfin_url,
            token: &settings.jellyfin_token,
            never_check: settings.jellyfin_disable_cert_check,
            custom_certificate: settings.jellyfin_use_custom_certificate,
            link: &settings.homepage_jellyfin_link,
            open_tab: !settings.jellyfin_tab_name.is_empty(),
            tab_name: &settings.jellyfin_tab_name,
        },
    }
}

/// What a request for a part needs: the server, and what the caller sees.
struct Asked {
    kind: Kind,
    settings: std::sync::Arc<Settings>,
    url: String,
    show_names: bool,
    /// The name of the caller in Organizr. A guest has the name "Guest", as in the PHP
    /// application.
    name: String,
    fresh: String,
}

impl Asked {
    fn new(state: &AppState, caller: &Caller, (kind, part): (Kind, Part)) -> Result<Self, Refusal> {
        let settings = state.settings();
        permissions(kind, part).check(&settings, caller)?;
        let show_names = settings.homepage_show_stream_names
            && u32::from(caller.level().get()) <= settings.homepage_show_stream_names_auth;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let name = caller
            .user()
            .map_or("Guest", |user| user.authenticated.user.username.as_str())
            .to_owned();
        Ok(Self {
            kind,
            url: service_url(install(&settings, kind).address),
            show_names,
            name,
            // The time makes the address of a picture new, so it is fetched again.
            fresh: format!("{now:x}"),
            settings,
        })
    }

    fn server(&self, timeout: Option<Duration>) -> Server<'_> {
        let install = install(&self.settings, self.kind);
        Server {
            kind: self.kind,
            url: &self.url,
            token: install.token,
            timeout,
            never_check_certificate: install.never_check,
        }
    }

    fn context<'a>(&'a self, is_cached: &'a (dyn Fn(&str) -> bool + Sync)) -> ItemContext<'a> {
        let install = install(&self.settings, self.kind);
        let scale = self.settings.cache_image_size;
        ItemContext {
            kind: self.kind,
            show_names: self.show_names,
            link: install.link,
            open_tab: install.open_tab,
            tab_name: install.tab_name,
            poster: (200 * scale, 300 * scale),
            wide: (1200 * scale, 675 * scale),
            fresh: &self.fresh,
            is_cached,
        }
    }

    /// The answer when the server refused a request, with the text of the PHP
    /// application.
    fn refused(&self) -> Refusal {
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{} Error Occurred", self.kind.name()),
        )
    }

    /// The answer when the server cannot be reached. The text names no address.
    fn unreachable(&self, error: &EmbyError) -> Refusal {
        tracing::warn!(%error, server = self.kind.name(), "the media server gave no answer");
        Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
    }

    /// The client for the requests to the server.
    fn http(&self, state: &AppState) -> Result<Outbound, Refusal> {
        state
            .outbound_for(install(&self.settings, self.kind).custom_certificate)
            .ok_or_else(no_requests)
    }

    /// The id of the user of the server whose eyes the lists use.
    async fn user(&self, state: &AppState, server: &Server<'_>) -> Result<String, Refusal> {
        let http = &self.http(state)?;
        let users = server
            .users(http)
            .await
            .map_err(|error| self.unreachable(&error))?
            .ok_or_else(|| self.refused())?;
        // Without a user, the PHP application asks for the items of the user 0, which
        // the server refuses.
        emby::user_of(&users, Some(&self.name)).ok_or_else(|| self.refused())
    }
}

/// Whether the picture cache holds a fresh picture of the server with the given key.
fn cache_check(kind: Kind, cache: Option<&PictureCache>) -> impl Fn(&str) -> bool + '_ {
    move |key: &str| cache.is_some_and(|cache| cache.is_fresh(&format!("{}-{key}", kind.source())))
}

/// The answer around the items, as the PHP application sends it.
fn answer(items: &[Value]) -> Envelope<Value> {
    Envelope::success(json!({ "content": items }))
}

/// What every handler takes from the request.
pub struct Request<'a> {
    /// The state of the application.
    pub state: &'a AppState,
    /// Who asks.
    pub caller: &'a Caller,
}

impl Request<'_> {
    fn asked(&self, kind: Kind, part: Part) -> Result<Asked, Refusal> {
        Asked::new(self.state, self.caller, (kind, part))
    }
}

/// What plays on the server now.
async fn streams(request: Request<'_>, kind: Kind) -> Result<Envelope<Value>, Refusal> {
    let asked = request.asked(kind, Part::Streams)?;
    let http = &asked.http(request.state)?;
    let timeout = time_limit(&asked.settings.homepage_stream_refresh.to_string());
    let sessions = asked
        .server(timeout)
        .sessions(http)
        .await
        .map_err(|error| asked.unreachable(&error))?
        .ok_or_else(|| asked.refused())?;
    let is_cached = cache_check(kind, request.state.picture_cache());
    Ok(answer(&emby::items(&sessions, &asked.context(&is_cached))))
}

/// The items that were added last, and that the user of the server with the name of
/// the caller did not play.
async fn recent(request: Request<'_>, kind: Kind) -> Result<Envelope<Value>, Refusal> {
    let asked = request.asked(kind, Part::Recent)?;
    let http = &asked.http(request.state)?;
    let timeout = time_limit(&asked.settings.homepage_recent_refresh.to_string());
    let server = asked.server(timeout);
    let user = asked.user(request.state, &server).await?;
    let added = server
        .latest(http, &user, asked.settings.homepage_recent_limit)
        .await
        .map_err(|error| asked.unreachable(&error))?
        .ok_or_else(|| asked.refused())?;
    let is_cached = cache_check(kind, request.state.picture_cache());
    Ok(answer(&emby::items(&added, &asked.context(&is_cached))))
}

/// The details of the item whose key the body names.
async fn metadata(
    request: Request<'_>,
    kind: Kind,
    data: &ApiData,
) -> Result<Envelope<Value>, Refusal> {
    let asked = request.asked(kind, Part::Metadata)?;
    let key = match data.field("key") {
        Some(Value::String(key)) => key.clone(),
        Some(Value::Number(key)) => key.to_string(),
        _ => String::new(),
    };
    // The key is the id of an item. Anything else could name another path.
    if !emby::is_id(&key) || key == "0" {
        return Err(Refusal::unprocessable(&format!(
            "{} Metadata key is not defined",
            kind.name()
        )));
    }
    let http = &asked.http(request.state)?;
    let server = asked.server(Some(DETAILS_TIME_LIMIT));
    let user = asked.user(request.state, &server).await?;
    let details = server
        .details(http, &user, &key, asked.settings.homepage_recent_limit)
        .await
        .map_err(|error| asked.unreachable(&error))?
        .ok_or_else(|| asked.refused())?;
    let is_cached = cache_check(kind, request.state.picture_cache());
    let has_name = details.get("Name").is_some_and(|name| !name.is_null());
    let item = if has_name {
        emby::item(&details, &asked.context(&is_cached))
    } else {
        None
    };
    Ok(answer(item.as_slice()))
}

/// The server that a `source` of `GET /homepage/image` names.
pub fn kind_of(source: &str) -> Option<Kind> {
    match source {
        "emby" => Some(Kind::Emby),
        "jellyfin" => Some(Kind::Jellyfin),
        _ => None,
    }
}

/// `GET /homepage/image` for a picture of an Emby or a Jellyfin server, at most as
/// large as the given size. The server keeps the picture for a week.
///
/// The query names the id of the item (`img`), the type of the picture (`type`), the
/// size (`width`, `height`) and the key of the picture in the cache (`key`). A `$` in
/// the key asks for the picture anew.
pub async fn image(
    request: Request<'_>,
    kind: Kind,
    query: &HashMap<String, String>,
    not_found: &(dyn Fn() -> Refusal + Sync),
) -> Result<Response, Refusal> {
    let field = |name: &str| query.get(name).map_or_default(String::as_str);
    let (key, refresh) = match field("key").split_once('$') {
        Some((key, _)) => (key, true),
        None => (field("key"), false),
    };
    let side = |name: &str| {
        field(name)
            .parse::<u32>()
            .ok()
            .filter(|side| (1..=LARGEST_SIDE).contains(side))
    };
    // The id and the type go into the path of a request to the server.
    let (item, picture) = (field("img"), field("type"));
    let (true, Some(width), Some(height)) = (
        emby::is_id(item) && PICTURE_TYPES.contains(&picture),
        side("width"),
        side("height"),
    ) else {
        return Err(not_found());
    };
    // The name in the cache comes from the picture that is fetched, not from the key
    // alone, so one picture cannot take the place of another (D-153).
    let name = emby::picture_name(item, picture, (width, height), key)
        .map(|name| format!("{}-{name}", kind.source()))
        .filter(|name| picture_cache::is_cache_name(name))
        .ok_or_else(not_found)?;
    let asked = request.asked(kind, Part::Metadata)?;
    let cache = request.state.picture_cache().ok_or_else(not_found)?;
    if !refresh
        && cache.is_fresh(&name)
        && let Some(content) = cache.read(&name).await
    {
        return picture_cache::picture_response(content).ok_or_else(not_found);
    }
    let http = &asked.http(request.state)?;
    let content = asked
        .server(None)
        .picture(http, (item, picture), (width, height), PICTURE_LIMIT)
        .await
        .map_err(|error| {
            tracing::warn!(%error, server = kind.name(), "a picture of the media server could not be loaded");
            not_found()
        })?
        .filter(|content| picture_cache::picture_type(content).is_some())
        .ok_or_else(not_found)?;
    if let Err(error) = cache.keep(&name, &content).await {
        tracing::warn!(%error, "a picture of the media server could not be kept");
    }
    picture_cache::picture_response(content).ok_or_else(not_found)
}

/// Makes the handler of a route that lists items of one of the two servers.
macro_rules! list_route {
    ($(#[$doc:meta])* $name:ident, $part:ident, $kind:expr) => {
        $(#[$doc])*
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<Caller>,
        ) -> Result<Envelope<Value>, Refusal> {
            let request = Request {
                state: &state,
                caller: &caller,
            };
            $part(request, $kind).await
        }
    };
}

/// Makes the handler of the route for the details of an item.
macro_rules! details_route {
    ($(#[$doc:meta])* $name:ident, $kind:expr) => {
        $(#[$doc])*
        pub async fn $name(
            State(state): State<AppState>,
            Extension(caller): Extension<Caller>,
            data: ApiData,
        ) -> Result<Envelope<Value>, Refusal> {
            let request = Request {
                state: &state,
                caller: &caller,
            };
            metadata(request, $kind, &data).await
        }
    };
}

list_route!(
    /// `GET /homepage/emby/streams`: what plays on the Emby server now.
    emby_streams,
    streams,
    Kind::Emby
);
list_route!(
    /// `GET /homepage/emby/recent`: the items that were added to the Emby server last.
    emby_recent,
    recent,
    Kind::Emby
);
details_route!(
    /// `POST /homepage/emby/metadata`: the details of the item whose key the body names.
    emby_metadata,
    Kind::Emby
);
list_route!(
    /// `GET /homepage/jellyfin/streams`: what plays on the Jellyfin server now.
    jellyfin_streams,
    streams,
    Kind::Jellyfin
);
list_route!(
    /// `GET /homepage/jellyfin/recent`: the items that were added to the Jellyfin
    /// server last.
    jellyfin_recent,
    recent,
    Kind::Jellyfin
);
details_route!(
    /// `POST /homepage/jellyfin/metadata`: the details of the item whose key the body
    /// names.
    jellyfin_metadata,
    Kind::Jellyfin
);
