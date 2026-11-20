//! The Emby live television item: the timers and the recordings of the live
//! television of the Emby server.

use std::time::SystemTime;

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use organizr_integrations::emby::{Kind, Server};
use organizr_integrations::emby_live_tv::{self, Shown};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// What must hold before a caller gets the item, and before an Admin runs its test.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageEmbyLiveTVTrackerEnabled".to_owned()],
        auth: vec!["homepageEmbyLiveTVTrackerAuth".to_owned()],
        not_empty: vec!["embyURL".to_owned(), "embyToken".to_owned()],
    }
}

/// How the interface shows the item.
pub fn options(settings: &Settings) -> Value {
    json!({
        "refreshMinutes": settings.homepage_emby_live_tv_tracker_refresh,
        "compact": settings.homepage_emby_live_tv_tracker_compact_view,
        "showDuration": settings.homepage_emby_live_tv_tracker_show_duration,
        "showSeriesInfo": settings.homepage_emby_live_tv_tracker_show_series_info,
        "showUserInfo": settings.homepage_emby_live_tv_tracker_show_user_info,
        "maxItems": settings.homepage_emby_live_tv_tracker_max_items,
        "showCompleted": settings.homepage_emby_live_tv_tracker_show_completed,
        "maxCompletedItems": settings.homepage_emby_live_tv_tracker_max_completed_items,
    })
}

/// The address of the Emby server, complete.
pub fn address(settings: &Settings, headers: &HeaderMap, client: &Client) -> String {
    service_url(&settings.emby_url, headers, client)
}

/// The Emby server of the settings, at its complete address.
pub fn server<'a>(settings: &'a Settings, url: &'a str) -> Server<'a> {
    Server {
        kind: Kind::Emby,
        url,
        token: &settings.emby_token,
        timeout: None,
        never_check_certificate: settings.emby_disable_cert_check,
    }
}

fn now() -> Timestamp {
    Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH)
}

/// `GET /homepage/embyLiveTVTracker/stats`: the numbers of the timers and of the
/// recordings.
pub async fn stats(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    permissions().check(&settings, &caller)?;
    let http = state.outbound().ok_or_else(no_requests)?;
    let url = address(&settings, &headers, &client);
    let numbers =
        emby_live_tv::stats(http, &server(&settings, &url), &TimeZone::system(), now()).await;
    Ok(Envelope::success(numbers).with_message("LiveTV stats retrieved successfully"))
}

/// `GET /homepage/embyLiveTVTracker/activity`: the timers, and the recordings that
/// are made.
pub async fn activity(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Value>, Refusal> {
    let settings = state.settings();
    // The PHP application has one answer for every reason of a refusal here.
    if !permissions().allow(&settings, &caller) {
        return Err(Refusal::Request(
            StatusCode::FORBIDDEN,
            "Permission denied".to_owned(),
        ));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let url = address(&settings, &headers, &client);
    let most = |number: u32| usize::try_from(number).unwrap_or(usize::MAX);
    let shown = Shown {
        timers: most(settings.homepage_emby_live_tv_tracker_max_items),
        completed: settings.homepage_emby_live_tv_tracker_show_completed,
        recordings: most(settings.homepage_emby_live_tv_tracker_max_completed_items),
        users: settings.homepage_emby_live_tv_tracker_show_user_info,
    };
    let lists = emby_live_tv::activity(http, &server(&settings, &url), shown, now()).await;
    Ok(Envelope::success(lists).with_message("LiveTV activity retrieved successfully"))
}
