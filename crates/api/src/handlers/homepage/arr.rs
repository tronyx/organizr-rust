//! Sonarr, Radarr and Lidarr on the homepage: the calendar, and the download queues of
//! Sonarr and Radarr.

use std::time::{Duration, SystemTime};

use axum::Extension;
use axum::extract::{RawQuery, State};
use axum::http::HeaderMap;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use organizr_integrations::arr::{
    Events, Install, Kind, Links, Releases, lidarr_events, radarr_events, sonarr_events,
};
use organizr_integrations::http::time_limit;
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, calendar_more, no_requests, service_url, trakt};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::picture_cache::Source;
use crate::state::AppState;

/// The settings of one of the three applications.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the switches are settings of PHP, each with its own meaning"
)]
struct App<'a> {
    /// The name in the keys of the settings: `Sonarr`, `Radarr` or `Lidarr`.
    name: &'static str,
    urls: &'a str,
    keys: &'a str,
    never_check_certificate: bool,
    /// Whether the requests also trust the authority that an Admin uploaded.
    custom_certificate: bool,
    unmonitored: bool,
    /// The time limit of a request for the calendar.
    calendar_limit: Option<Duration>,
    /// The time limit of a request for the queue.
    queue_limit: Option<Duration>,
    calendar_link: &'a str,
    frame: &'a str,
    show_link: bool,
}

fn app(settings: &Settings, kind: Kind) -> App<'_> {
    match kind {
        Kind::Sonarr => App {
            name: "Sonarr",
            urls: &settings.sonarr_url,
            keys: &settings.sonarr_token,
            never_check_certificate: settings.sonarr_disable_cert_check,
            custom_certificate: settings.sonarr_use_custom_certificate,
            unmonitored: settings.sonarr_unmonitored,
            calendar_limit: None,
            queue_limit: time_limit(&settings.homepage_sonarr_queue_refresh.to_string()),
            calendar_link: &settings.sonarr_calendar_link,
            frame: &settings.sonarr_frame_target,
            show_link: settings.sonarr_icon,
        },
        Kind::Radarr => App {
            name: "Radarr",
            urls: &settings.radarr_url,
            keys: &settings.radarr_token,
            never_check_certificate: settings.radarr_disable_cert_check,
            custom_certificate: settings.radarr_use_custom_certificate,
            unmonitored: settings.radarr_unmonitored,
            calendar_limit: time_limit(&settings.homepage_radarr_queue_refresh.to_string()),
            queue_limit: time_limit(&settings.homepage_radarr_queue_refresh.to_string()),
            calendar_link: &settings.radarr_calendar_link,
            frame: &settings.radarr_frame_target,
            show_link: settings.radarr_icon,
        },
        Kind::Lidarr => App {
            name: "Lidarr",
            urls: &settings.lidarr_url,
            keys: &settings.lidarr_token,
            never_check_certificate: settings.lidarr_disable_cert_check,
            custom_certificate: settings.lidarr_use_custom_certificate,
            unmonitored: false,
            calendar_limit: None,
            queue_limit: None,
            calendar_link: &settings.lidarr_calendar_link,
            frame: &settings.lidarr_frame_target,
            show_link: settings.lidarr_icon,
        },
    }
}

/// What must hold before a caller gets the calendar of an application.
pub fn calendar_permissions(kind: Kind) -> Permissions {
    let name = match kind {
        Kind::Sonarr => "Sonarr",
        Kind::Radarr => "Radarr",
        Kind::Lidarr => "Lidarr",
    };
    let lower = name.to_lowercase();
    Permissions {
        enabled: vec![format!("homepage{name}Enabled")],
        auth: vec![format!("homepage{name}Auth")],
        not_empty: vec![format!("{lower}URL"), format!("{lower}Token")],
    }
}

/// What must hold before a caller gets the download queue of an application.
pub fn queue_permissions(kind: Kind) -> Permissions {
    let mut permissions = calendar_permissions(kind);
    let name = if kind == Kind::Sonarr {
        "Sonarr"
    } else {
        "Radarr"
    };
    permissions
        .enabled
        .push(format!("homepage{name}QueueEnabled"));
    permissions.auth.push(format!("homepage{name}QueueAuth"));
    permissions
}

/// The installs of an application: the addresses and the keys of the settings come
/// with commas between them, and each address goes with the key at the same place.
fn installs(app: &App<'_>) -> Vec<(String, String)> {
    let keys: Vec<&str> = app.keys.split(',').collect();
    app.urls
        .split(',')
        .enumerate()
        .filter_map(|(index, url)| {
            let key = keys.get(index)?;
            Some((service_url(url), (*key).to_owned()))
        })
        .collect()
}

/// A date for the query of a calendar: today, moved by a number of days.
fn day(now: Timestamp, zone: &TimeZone, days: i64) -> String {
    let moved = now
        .to_zoned_datetime(zone.clone())
        .date()
        .checked_add(jiff::Span::new().days(days))
        .unwrap_or_else(|_| now.to_zoned_datetime(zone.clone()).date());
    moved.strftime("%Y-%m-%d").to_string()
}

/// A field of a query, when it holds a value.
fn query_field<'a>(query: Option<&'a str>, name: &str) -> Option<&'a str> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value)
}

/// The dates of the calendar: from the query, or from the settings around today.
fn range(
    settings: &Settings,
    query: Option<&str>,
    now: Timestamp,
    zone: &TimeZone,
) -> (String, String) {
    let start = query_field(query, "start").map_or_else(
        || day(now, zone, -i64::from(settings.calendar_start)),
        str::to_owned,
    );
    let end = query_field(query, "end").map_or_else(
        || day(now, zone, i64::from(settings.calendar_end)),
        str::to_owned,
    );
    (start, end)
}

/// The events of every install of an application. An install that gives no usable
/// answer is left out.
async fn events_of(
    state: &AppState,
    kind: Kind,
    (start, end): (&str, &str),
) -> Result<Vec<Value>, Refusal> {
    let settings = state.settings();
    let app = app(&settings, kind);
    let http = &state
        .outbound_for(app.custom_certificate)
        .ok_or_else(no_requests)?;
    let now = Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH);
    let zone = TimeZone::system();
    let base = if app.calendar_link.is_empty() {
        app.urls.split(',').next().unwrap_or_default()
    } else {
        app.calendar_link
    };
    let links = Links {
        base,
        frame: app.frame,
        show_link: app.show_link,
    };

    let mut all = Vec::new();
    for (number, (url, key)) in installs(&app).iter().enumerate() {
        let install = Install {
            kind,
            url,
            key,
            timeout: app.calendar_limit,
            never_check_certificate: app.never_check_certificate,
        };
        let entries = match install.calendar(http, start, end, app.unmonitored).await {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(%error, application = app.name, "a calendar could not be read");
                continue;
            }
        };
        let Events { events, pictures } = match kind {
            Kind::Sonarr => sonarr_events(&entries, number, links, now),
            Kind::Radarr => {
                let releases = Releases {
                    physical: settings.radarr_physical_release,
                    digital: settings.radarr_digital_release,
                    cinema: settings.radarr_cinema_release,
                };
                radarr_events(&entries, number, links, releases, now, &zone)
            }
            Kind::Lidarr => lidarr_events(&entries, number, links, now),
        };
        if let Some(cache) = state.picture_cache() {
            for picture in pictures {
                // A path is a file of the install, which wants the key.
                let source = if picture.source.contains("://") {
                    Source {
                        url: picture.source,
                        key: None,
                        never_check_certificate: false,
                    }
                } else {
                    Source {
                        url: format!("{}{}", url.trim_end_matches('/'), picture.source),
                        key: Some(key.clone()),
                        never_check_certificate: app.never_check_certificate,
                    }
                };
                cache.fetch_later(http, &picture.name, source);
            }
        }
        all.extend(events);
    }
    Ok(all)
}

async fn calendar_of(
    state: &AppState,
    caller: &Caller,
    query: Option<&str>,
    kind: Kind,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    let settings = state.settings();
    calendar_permissions(kind).check(&settings, caller)?;
    let now = Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH);
    let (start, end) = range(&settings, query, now, &TimeZone::system());
    let events = events_of(state, kind, (&start, &end)).await?;
    Ok(Envelope::success(events))
}

/// `GET /homepage/sonarr/calendar`: the episodes of the calendar of Sonarr.
pub async fn sonarr_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    calendar_of(&state, &caller, query.as_deref(), Kind::Sonarr).await
}

/// `GET /homepage/radarr/calendar`: the films of the calendar of Radarr.
pub async fn radarr_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    calendar_of(&state, &caller, query.as_deref(), Kind::Radarr).await
}

/// `GET /homepage/lidarr/calendar`: the albums of the calendar of Lidarr.
pub async fn lidarr_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    RawQuery(query): RawQuery,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    calendar_of(&state, &caller, query.as_deref(), Kind::Lidarr).await
}

/// Whether the caller gets the calendar of at least one source: an application, the
/// account at Trakt, or the iCal addresses.
pub fn sees_a_calendar(settings: &Settings, caller: &Caller) -> bool {
    [Kind::Sonarr, Kind::Lidarr, Kind::Radarr]
        .into_iter()
        .map(calendar_permissions)
        .chain([
            calendar_more::sickrage_permissions(),
            calendar_more::couchpotato_permissions(),
            trakt::permissions(),
            calendar_more::ical_permissions(),
        ])
        .any(|permissions| permissions.allow(settings, caller))
}

/// `GET /homepage/calendar`: the events of every application whose calendar the
/// caller gets and of the account at Trakt, and the events of the iCal addresses
/// apart from them. The answer is a
/// success also when the caller gets none, as in the PHP application.
pub async fn calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    let now = Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH);
    let (start, end) = range(&settings, None, now, &TimeZone::system());
    let mut events = Vec::new();
    for kind in [Kind::Sonarr, Kind::Lidarr, Kind::Radarr] {
        if calendar_permissions(kind).allow(&settings, &caller) {
            events.extend(events_of(&state, kind, (&start, &end)).await?);
        }
    }
    if calendar_more::sickrage_permissions().allow(&settings, &caller) {
        events.extend(calendar_more::sickrage_events(&state).await?);
    }
    if calendar_more::couchpotato_permissions().allow(&settings, &caller) {
        events.extend(calendar_more::couchpotato_events(&state).await?);
    }
    if trakt::permissions().allow(&settings, &caller) {
        // A calendar of Trakt that cannot be read is left out, as in the PHP
        // application.
        if let Ok(found) = trakt::events(&state, &headers, &client).await {
            events.extend(found);
        }
    }
    let ical = calendar_more::ical_part(&state, &caller).await;
    Ok(Envelope::success(json!({ "ical": ical, "events": events })))
}

async fn queue_of(
    state: &AppState,
    caller: &Caller,
    kind: Kind,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    queue_permissions(kind).check(&settings, caller)?;
    let app = app(&settings, kind);
    let http = &state
        .outbound_for(app.custom_certificate)
        .ok_or_else(no_requests)?;
    let mut items = Vec::new();
    for (url, key) in &installs(&app) {
        let install = Install {
            kind,
            url,
            key,
            timeout: app.queue_limit,
            never_check_certificate: app.never_check_certificate,
        };
        match install.queue(http).await {
            Ok(records) => items.extend(records),
            Err(error) => {
                tracing::warn!(%error, application = app.name, "a download queue could not be read");
            }
        }
    }
    Ok(Envelope::success(json!({
        "content": { "queueItems": items, "historyItems": false },
    })))
}

/// `GET /homepage/sonarr/queue`: the downloads in the queue of Sonarr.
pub async fn sonarr_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    queue_of(&state, &caller, Kind::Sonarr).await
}

/// `GET /homepage/radarr/queue`: the downloads in the queue of Radarr.
pub async fn radarr_queue(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Value>, Refusal> {
    queue_of(&state, &caller, Kind::Radarr).await
}
