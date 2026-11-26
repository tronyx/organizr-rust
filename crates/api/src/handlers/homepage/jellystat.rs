//! The JellyStat item: the statistics of an Emby or a Jellyfin server, as JellyStat
//! counts them, or the pages of JellyStat in a frame.

use std::time::Duration;

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use jiff::ZonedDateTime;
use organizr_integrations::emby::{self, Kind, Server};
use organizr_integrations::http::Outbound;
use organizr_integrations::jellystat::{self, Install, MediaServer, Shown};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::emby as media;
use super::{Permissions, no_requests, service_url};
use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::php_url;
use crate::state::AppState;

/// How long a media server has to answer the question for the details of an item.
const DETAILS_TIME_LIMIT: Duration = Duration::from_secs(60);

/// Whether the item shows the pages of JellyStat in a frame.
fn is_frame(settings: &Settings) -> bool {
    settings.homepage_jelly_stat_display_mode == "iframe"
}

/// What must hold before a caller gets the item. The statistics need the API key,
/// and the frame does not.
pub fn permissions(settings: &Settings) -> Permissions {
    let mut permissions = details_permissions();
    if !is_frame(settings) {
        permissions.not_empty.push("jellyStatApikey".to_owned());
    }
    permissions
}

/// What must hold before a caller gets the details of an item.
pub fn details_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageJellyStatEnabled".to_owned()],
        auth: vec!["homepageJellyStatAuth".to_owned()],
        not_empty: vec!["jellyStatURL".to_owned()],
    }
}

/// The address of JellyStat for the browser. An address that starts with a slash is
/// a path on the host of Organizr, which the browser completes.
fn browser_address(settings: &Settings) -> String {
    let address: String = settings
        .jelly_stat_url
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    if address.is_empty() || address.starts_with('/') {
        return address;
    }
    php_url::qualify(address.trim_end_matches('/'), false, "")
}

/// How the interface shows the item.
pub fn options(settings: &Settings) -> Value {
    json!({
        "mode": if is_frame(settings) { "iframe" } else { "native" },
        "url": browser_address(settings),
        "frameHeight": settings.homepage_jelly_stat_iframe_height,
        "frameScrolling": settings.homepage_jelly_stat_iframe_scrolling,
        "refreshMinutes": settings.homepage_jelly_stat_refresh,
        "days": settings.homepage_jelly_stat_days,
        "maxItems": settings.homepage_jelly_stat_max_items,
        "showLibraries": settings.homepage_jelly_stat_show_libraries,
        "showUsers": settings.homepage_jelly_stat_show_users,
        "showMostWatched": settings.homepage_jelly_stat_show_most_watched,
        "showMovies": settings.homepage_jelly_stat_show_most_watched_movies,
        "showShows": settings.homepage_jelly_stat_show_most_watched_shows,
        "showMusic": settings.homepage_jelly_stat_show_most_listened_music,
        "mostWatchedCount": settings.homepage_jelly_stat_most_watched_count,
    })
}

/// The address that the server asks: the internal one when the settings have one.
pub fn api_address(settings: &Settings) -> String {
    let address = if settings.jelly_stat_internal_url.is_empty() {
        &settings.jelly_stat_url
    } else {
        &settings.jelly_stat_internal_url
    };
    service_url(address)
}

fn failed(message: impl Into<String>) -> Refusal {
    Refusal::Request(StatusCode::INTERNAL_SERVER_ERROR, message.into())
}

/// `GET /homepage/jellystat`: the libraries, the users and the most played items of
/// the days of the settings.
pub async fn data(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    if !permissions(&settings).allow(&settings, &caller) {
        return Err(Refusal::not_authorized(
            "User not approved to view this homepage item - check plugin configuration",
        ));
    }
    if settings.jelly_stat_url.is_empty() || settings.jelly_stat_apikey.is_empty() {
        return Err(failed("JellyStat URL or API key not configured"));
    }
    let http = &state
        .outbound_for(settings.jelly_stat_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let url = api_address(&settings);
    let install = Install {
        url: &url,
        key: &settings.jelly_stat_apikey,
        never_check_certificate: settings.jelly_stat_disable_cert_check,
    };
    let unreachable = |error: String| {
        tracing::warn!(%error, "the statistics of JellyStat could not be read");
        failed(format!("Failed to fetch JellyStat data: {error}"))
    };
    let now = ZonedDateTime::now();
    let days = settings.homepage_jelly_stat_days;
    let start = now
        .date()
        .checked_sub(jiff::Span::new().days(i64::from(days)))
        .unwrap_or_else(|_| now.date())
        .strftime("%Y-%m-%d")
        .to_string();

    let libraries = install.libraries(http).await.map_err(unreachable)?;
    let history = install.history(http, &start).await.map_err(unreachable)?;
    let shown = Shown {
        days,
        movies: settings.homepage_jelly_stat_show_most_watched_movies,
        shows: settings.homepage_jelly_stat_show_most_watched_shows,
        music: settings.homepage_jelly_stat_show_most_listened_music,
        most: usize::try_from(settings.homepage_jelly_stat_most_watched_count)
            .unwrap_or(usize::MAX),
    };
    let statistics = jellystat::statistics(
        libraries.as_deref(),
        &history,
        shown,
        &now.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
    );
    Ok(Envelope::success(statistics).with_message("JellyStat data retrieved successfully"))
}

/// The media server whose items JellyStat counts: Jellyfin when the homepage shows
/// it, or else Emby.
fn media_server(settings: &Settings) -> Option<Kind> {
    let is_ready =
        |is_on: bool, address: &str, token: &str| is_on && !address.is_empty() && !token.is_empty();
    if is_ready(
        settings.homepage_jellyfin_enabled,
        &settings.jellyfin_url,
        &settings.jellyfin_token,
    ) {
        Some(Kind::Jellyfin)
    } else if is_ready(
        settings.homepage_emby_enabled,
        &settings.emby_url,
        &settings.emby_token,
    ) {
        Some(Kind::Emby)
    } else {
        None
    }
}

/// What the media server knows about an item, as its administrator sees it. Nothing
/// when the server gives no answer that can be used.
async fn details_of_server(
    http: &Outbound,
    server: &Server<'_>,
    key: &str,
    limit: u32,
) -> Option<Value> {
    let users = match server.users(http).await {
        Ok(users) => users?,
        Err(error) => {
            tracing::info!(%error, "the media server gave no users for the details of JellyStat");
            return None;
        }
    };
    let user = emby::user_of(&users, None)?;
    let details = server.details(http, &user, key, limit).await.ok()??;
    details
        .as_object()
        .is_some_and(|fields| !fields.is_empty())
        .then_some(details)
}

/// `POST /homepage/jellystat/metadata`: the details of the item whose key the body
/// names, in the form of the items of the media servers.
pub async fn metadata(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    data: ApiData,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    if !details_permissions().allow(&settings, &caller) {
        return Err(Refusal::not_authorized(
            "Not authorized for JellyStat metadata",
        ));
    }
    let key = match data.field("key") {
        Some(Value::String(key)) => key.clone(),
        Some(Value::Number(key)) => key.to_string(),
        _ => String::new(),
    };
    // The key is the id of an item. Anything else could name another path.
    if !emby::is_id(&key) || key == "0" {
        return Err(Refusal::unprocessable(
            "JellyStat metadata key is not defined",
        ));
    }
    let http = &state
        .outbound_for(settings.jelly_stat_use_custom_certificate)
        .ok_or_else(no_requests)?;

    // The media server is asked as its administrator, so only for a caller who has the
    // level of its item on the homepage. Another caller gets what JellyStat knows.
    let kind = media_server(&settings)
        .filter(|kind| media::permissions(*kind, media::Part::Metadata).allow(&settings, &caller));
    let media_url = kind.map(|kind| service_url(media::install(&settings, kind).address));
    let mut details = None;
    if let (Some(kind), Some(url)) = (kind, media_url.as_deref()) {
        let install = media::install(&settings, kind);
        let server = Server {
            kind,
            url,
            token: install.token,
            timeout: Some(DETAILS_TIME_LIMIT),
            never_check_certificate: install.never_check,
        };
        // The media server has its own setting for the custom certificate, as in PHP.
        let media_http = state
            .outbound_for(install.custom_certificate)
            .ok_or_else(no_requests)?;
        details =
            details_of_server(&media_http, &server, &key, settings.homepage_recent_limit).await;
    }
    if details.is_none() {
        let url = api_address(&settings);
        let install = Install {
            url: &url,
            key: &settings.jelly_stat_apikey,
            never_check_certificate: settings.jelly_stat_disable_cert_check,
        };
        details = install.item(http, &key).await;
    }

    let links = kind.map(|kind| media::install(&settings, kind));
    let server = kind
        .zip(links.as_ref())
        .zip(media_url.as_deref())
        .map(|((kind, links), url)| MediaServer {
            kind,
            url,
            link: links.link.trim(),
            tab_name: links.tab_name,
            open_tab: links.open_tab,
        });
    let item = jellystat::details_item(
        &key,
        details.as_ref(),
        server.as_ref(),
        &browser_address(&settings),
    );
    Ok(Envelope::success(json!({ "content": [item] })))
}
