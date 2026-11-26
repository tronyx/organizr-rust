//! More sources of the calendar of the homepage: SickRage, CouchPotato, and
//! calendars in the iCalendar format.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::StatusCode;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use organizr_integrations::http::Outbound;
use organizr_integrations::ical::{self, Window};
use organizr_integrations::{couchpotato, pihole, sickrage};
use organizr_settings::Settings;
use serde_json::Value;

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::picture_cache::Source;
use crate::state::AppState;

/// What must hold before a caller gets the episodes of SickRage.
pub fn sickrage_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageSickrageEnabled".to_owned()],
        auth: vec!["homepageSickrageAuth".to_owned()],
        not_empty: vec!["sickrageURL".to_owned(), "sickrageToken".to_owned()],
    }
}

/// What must hold before a caller gets the films of CouchPotato.
pub fn couchpotato_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageCouchpotatoEnabled".to_owned()],
        auth: vec!["homepageCouchpotatoAuth".to_owned()],
        not_empty: vec!["couchpotatoURL".to_owned(), "couchpotatoToken".to_owned()],
    }
}

/// What must hold before a caller gets the events of the iCal addresses.
pub fn ical_permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageCalendarEnabled".to_owned()],
        auth: vec!["homepageCalendarAuth".to_owned()],
        not_empty: vec!["calendariCal".to_owned()],
    }
}

/// The days around today that the settings name.
fn window(settings: &Settings) -> Window {
    Window {
        now: Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH),
        zone: TimeZone::system(),
        days_before: i64::from(settings.calendar_start),
        days_after: i64::from(settings.calendar_end),
    }
}

/// The installs of an application, each one with its complete address and its key.
fn installs(urls: &str, keys: &str) -> Vec<(String, String)> {
    pihole::installs(urls, keys)
        .into_iter()
        .map(|(url, key)| (service_url(url), key.to_owned()))
        .collect()
}

/// The episodes of every install of SickRage: the ones that were missed, the ones to
/// come, and the ones that were downloaded. An install that gives no usable answer
/// is left out.
pub async fn sickrage_events(state: &AppState) -> Result<Vec<Value>, Refusal> {
    let settings = state.settings();
    let http = &state
        .outbound_for(settings.sickrage_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let (now, zone) = (window(&settings).now, TimeZone::system());
    let mut all = Vec::new();
    let installs = installs(&settings.sickrage_url, &settings.sickrage_token);
    for (number, (url, key)) in installs.iter().enumerate() {
        let install = sickrage::Install {
            url,
            key,
            never_check_certificate: settings.sickrage_disable_cert_check,
        };
        let answers = async {
            Ok::<_, sickrage::SickRageError>((
                install.future(http).await?,
                install.history(http).await?,
            ))
        };
        match answers.await {
            Ok((future, history)) => {
                all.extend(sickrage::future_events(&future, number, now, &zone));
                all.extend(sickrage::history_events(&history, number));
            }
            Err(error) => {
                tracing::warn!(%error, application = "SickRage", "a calendar could not be read");
            }
        }
    }
    Ok(all)
}

/// The films of every install of CouchPotato whose release is in the days of the
/// settings. An install that gives no usable answer is left out.
pub async fn couchpotato_events(state: &AppState) -> Result<Vec<Value>, Refusal> {
    let settings = state.settings();
    let http = &state
        .outbound_for(settings.couchpotato_use_custom_certificate)
        .ok_or_else(no_requests)?;
    let window = window(&settings);
    let mut all = Vec::new();
    let installs = installs(&settings.couchpotato_url, &settings.couchpotato_token);
    for (number, (url, key)) in installs.iter().enumerate() {
        let install = couchpotato::Install {
            url,
            key,
            never_check_certificate: settings.couchpotato_disable_cert_check,
        };
        let found = match install.films(http).await {
            Ok(answer) => couchpotato::film_events(&answer, number, &window),
            Err(error) => {
                tracing::warn!(%error, application = "CouchPotato", "a calendar could not be read");
                continue;
            }
        };
        if let Some(cache) = state.picture_cache() {
            for picture in found.pictures {
                let source = Source {
                    url: picture.source,
                    key: None,
                    never_check_certificate: false,
                };
                cache.fetch_later(http, &picture.name, source);
            }
        }
        all.extend(found.events);
    }
    Ok(all)
}

/// Why the caller does not get the events of the iCal addresses, with the status and
/// the text of the PHP application. Nothing when the caller gets them.
fn ical_refusal(settings: &Settings, caller: &Caller) -> Option<Refusal> {
    let permissions = ical_permissions();
    let is_on = Permissions {
        enabled: permissions.enabled.clone(),
        ..Permissions::default()
    };
    let has_level = Permissions {
        auth: permissions.auth.clone(),
        ..Permissions::default()
    };
    if !is_on.allow(settings, caller) {
        return Some(Refusal::conflict("iCal homepage item is not enabled"));
    }
    if !has_level.allow(settings, caller) {
        return Some(Refusal::Request(
            StatusCode::UNAUTHORIZED,
            "User not approved to view this homepage item".to_owned(),
        ));
    }
    if !permissions.allow(settings, caller) {
        return Some(Refusal::unprocessable("iCal URL is not defined"));
    }
    None
}

/// How long the events of a feed are kept, so that a new look at the calendar neither
/// downloads nor reads the feed again (D-142).
const FEED_KEPT_FOR: Duration = Duration::from_mins(5);

/// The most feeds whose events are kept.
const MOST_FEEDS_KEPT: usize = 64;

/// A feed as the settings name it: its address and the days around today.
type FeedKey = (String, i64, i64);

/// The events of the feeds that were read a moment ago, with when they were read.
type KeptFeeds = HashMap<FeedKey, (Instant, Vec<Value>)>;

static KEPT: LazyLock<Mutex<KeptFeeds>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// The events of every iCal address of the settings. An address that gives no
/// calendar is left out.
async fn ical_events(settings: &Settings, http: &Outbound) -> Vec<Value> {
    let window = window(settings);
    let mut all = Vec::new();
    for (number, address) in settings.calendar_ical.split(',').enumerate() {
        if address.trim().is_empty() {
            continue;
        }
        let key = (address.to_owned(), window.days_before, window.days_after);
        if let Some(events) = kept(&key) {
            all.extend(events);
            continue;
        }
        // The address of a private calendar holds a key, so the log names the place
        // of the address in the setting only.
        let text = match ical::fetch(http, address).await {
            Ok(text) => text,
            Err(error) => {
                tracing::warn!(%error, number, "a calendar in the iCal format could not be read");
                continue;
            }
        };
        // Reading a feed and its repeats takes work, so it runs away from the tasks that
        // serve requests.
        let read = window.clone();
        let Ok(events) = tokio::task::spawn_blocking(move || ical::events(&text, &read)).await
        else {
            tracing::warn!(number, "a calendar in the iCal format could not be read");
            continue;
        };
        keep(key, events.clone());
        all.extend(events);
    }
    all
}

/// The events of a feed, when they were read less than [`FEED_KEPT_FOR`] ago.
fn kept(key: &FeedKey) -> Option<Vec<Value>> {
    let feeds = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    feeds
        .get(key)
        .filter(|(since, _)| since.elapsed() < FEED_KEPT_FOR)
        .map(|(_, events)| events.clone())
}

fn keep(key: FeedKey, events: Vec<Value>) {
    let mut feeds = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    if feeds.len() >= MOST_FEEDS_KEPT {
        feeds.retain(|_, (since, _)| since.elapsed() < FEED_KEPT_FOR);
        if feeds.len() >= MOST_FEEDS_KEPT {
            feeds.clear();
        }
    }
    feeds.insert(key, (Instant::now(), events));
}

/// The `ical` part of `GET /homepage/calendar`: the events of the iCal addresses, or
/// `false` for a caller who does not get them, as in the PHP application.
pub async fn ical_part(state: &AppState, caller: &Caller) -> Value {
    let settings = state.settings();
    if ical_refusal(&settings, caller).is_some() {
        return Value::Bool(false);
    }
    match state.outbound() {
        Some(http) => Value::Array(ical_events(&settings, http).await),
        None => Value::Bool(false),
    }
}

/// `GET /homepage/sickrage/calendar`: the episodes of SickRage.
pub async fn sickrage_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    sickrage_permissions().check(&state.settings(), &caller)?;
    let events = sickrage_events(&state).await?;
    Ok(Envelope::success(events))
}

/// `GET /homepage/couchpotato/calendar`: the films of CouchPotato.
pub async fn couchpotato_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    couchpotato_permissions().check(&state.settings(), &caller)?;
    let events = couchpotato_events(&state).await?;
    Ok(Envelope::success(events))
}

/// `GET /homepage/ical/calendar`: the events of the iCal addresses.
pub async fn ical_calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    let settings = state.settings();
    if let Some(refusal) = ical_refusal(&settings, &caller) {
        return Err(refusal);
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    Ok(Envelope::success(ical_events(&settings, http).await))
}
