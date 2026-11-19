//! Stand-ins for the applications whose data the homepage shows.
//!
//! The answers have the shape that the applications give. Every name and value in
//! them is made up.

use axum::Router;
use axum::extract::RawQuery;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};

use crate::services::StandIn;

/// The API keys of the two projects of the Healthchecks stand-in.
pub const HEALTH_KEYS: (&str, &str) = ("made-up-key-of-project-one", "made-up-key-of-project-two");

fn check(name: &str, tags: &str, status: &str) -> Value {
    json!({
        "name": name,
        "slug": name.to_lowercase().replace(' ', "-"),
        "tags": tags,
        "desc": format!("Notes of {name}"),
        "grace": 3600,
        "n_pings": 12,
        "status": status,
        "started": false,
        "last_ping": "2026-11-01T10:00:00+00:00",
        "next_ping": "2026-11-02T10:00:00+00:00",
        "manual_resume": false,
        "methods": "",
        "unique_key": format!("made-up-unique-key-of-{}", name.to_lowercase().replace(' ', "-")),
        "timeout": 86400,
    })
}

/// The checks of the project of a key.
fn checks_of(key: &str) -> Option<Vec<Value>> {
    if key == HEALTH_KEYS.0 {
        Some(vec![
            check("Nightly backup", "prod backup", "up"),
            check("Mail queue", "prod", "down"),
            check("Old job", "", "paused"),
        ])
    } else if key == HEALTH_KEYS.1 {
        Some(vec![
            check("Certificate renewal", "web", "grace"),
            check("Site map", "web prod", "down"),
        ])
    } else {
        None
    }
}

async fn health_checks(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    let key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let Some(checks) = checks_of(key) else {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "wrong api key"})),
        )
            .into_response();
    };
    // A check is listed when it has every tag that the query names.
    let wanted: Vec<String> = query
        .unwrap_or_default()
        .split('&')
        .filter_map(|pair| pair.strip_prefix("tag=").map(str::to_owned))
        .collect();
    let checks: Vec<Value> = checks
        .into_iter()
        .filter(|check| {
            let tags: Vec<&str> = check["tags"]
                .as_str()
                .unwrap_or_default()
                .split(' ')
                .collect();
            wanted.iter().all(|tag| tags.contains(&tag.as_str()))
        })
        .collect();
    axum::Json(json!({ "checks": checks })).into_response()
}

/// A stand-in for Healthchecks. The address for the settings is its URL with
/// `/api/v1/checks`.
pub async fn healthchecks() -> StandIn {
    StandIn::start(Router::new().route("/api/v1/checks/", get(health_checks))).await
}

/// The API key of the Uptime Kuma stand-in.
pub const KUMA_KEY: &str = "made-up-kuma-key";

/// `Basic` with an empty user name and [`KUMA_KEY`] as the password.
const KUMA_AUTHORIZATION: &str = "Basic Om1hZGUtdXAta3VtYS1rZXk=";

const KUMA_METRICS: &str = "# HELP monitor_status Monitor Status (1 = UP, 0= DOWN, 2= PENDING, 3= MAINTENANCE)\n\
# TYPE monitor_status gauge\n\
monitor_status{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 1\n\
monitor_status{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 0\n\
monitor_status{monitor_name=\"New one\",monitor_type=\"http\",monitor_url=\"https://new.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} 2\n\
# HELP monitor_response_time Monitor Response Time (ms)\n\
# TYPE monitor_response_time gauge\n\
monitor_response_time{monitor_name=\"Router\",monitor_type=\"ping\",monitor_url=\"null\",monitor_hostname=\"192.0.2.1\",monitor_port=\"null\"} 3\n\
monitor_response_time{monitor_name=\"Blog\",monitor_type=\"http\",monitor_url=\"https://blog.example.com\",monitor_hostname=\"null\",monitor_port=\"null\"} -1\n";

async fn kuma_metrics(headers: HeaderMap) -> Response {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if authorization == Some(KUMA_AUTHORIZATION) {
        (
            [(
                header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            KUMA_METRICS,
        )
            .into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Secure Area\"")],
        )
            .into_response()
    }
}

/// A stand-in for Uptime Kuma, with two monitors that were checked and one that was
/// not checked yet.
pub async fn uptime_kuma() -> StandIn {
    StandIn::start(Router::new().route("/metrics", get(kuma_metrics))).await
}

/// The API key of the stand-ins for Sonarr, Radarr and Lidarr.
pub const ARR_KEY: &str = "made-up-arr-key";

/// The first bytes of a JPEG picture, which is all that the stand-ins send as one.
pub const JPEG: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];

fn has_arr_key(headers: &HeaderMap) -> bool {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        == Some(ARR_KEY)
}

/// The answer of a route of the stand-ins: the given JSON for the right key.
fn for_arr_key(headers: &HeaderMap, body: Value) -> Response {
    if has_arr_key(headers) {
        axum::Json(body).into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Unauthorized"})),
        )
            .into_response()
    }
}

async fn arr_picture(headers: HeaderMap) -> Response {
    if has_arr_key(&headers) {
        ([(header::CONTENT_TYPE, "image/jpeg")], JPEG).into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn sonarr_calendar(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!([{
            "seriesId": 1,
            "seasonNumber": 2,
            "episodeNumber": 3,
            "title": "The One",
            "airDateUtc": "2020-01-05T20:00:00Z",
            "overview": "What happens.",
            "hasFile": false,
            "monitored": true,
            "series": {
                "title": "The Show",
                "tvdbId": 4711,
                "status": "continuing",
                "runtime": 45,
                "ratings": {"votes": 10, "value": 8.1},
                "genres": ["Comedy"],
                "seasons": [{"seasonNumber": 1}, {"seasonNumber": 2}],
                "images": [{"coverType": "fanart", "url": "/MediaCover/1/fanart.jpg"}],
            },
        }]),
    )
}

async fn sonarr_queue(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!({
            "page": 1,
            "pageSize": 1000,
            "totalRecords": 1,
            "records": [{
                "id": 31,
                "title": "The.Show.S02E03.1080p",
                "status": "downloading",
                "size": 1000,
                "sizeleft": 250,
                "timeleft": "00:10:00",
                "series": {"title": "The Show"},
                "episode": {"seasonNumber": 2, "episodeNumber": 3, "title": "The One"},
            }],
        }),
    )
}

/// A stand-in for Sonarr: one episode in the calendar whose fan art only Sonarr has,
/// and one download in the queue.
pub async fn sonarr() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v3/calendar", get(sonarr_calendar))
            .route("/api/v3/queue", get(sonarr_queue))
            .route("/MediaCover/1/fanart.jpg", get(arr_picture)),
    )
    .await
}

async fn radarr_calendar(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!([{
            "title": "The Film",
            "tmdbId": 815,
            "status": "released",
            "overview": "A film.",
            "runtime": 101,
            "hasFile": false,
            "year": 2020,
            "studio": "Studio",
            "genres": ["Drama"],
            "ratings": {"value": 7.2},
            // At noon, so the day is the same in the time zone of every test machine.
            "physicalRelease": "2020-01-07T12:00:00Z",
            "images": [{
                "coverType": "fanart",
                "url": "/MediaCover/5/fanart.jpg",
                "remoteUrl": "https://art.example.net/film.jpg",
            }],
        }]),
    )
}

async fn radarr_queue(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!({"page": 1, "pageSize": 1000, "totalRecords": 0, "records": []}),
    )
}

/// A stand-in for Radarr: one film in the calendar, and an empty queue.
pub async fn radarr() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v3/calendar", get(radarr_calendar))
            .route("/api/v3/queue", get(radarr_queue)),
    )
    .await
}

async fn lidarr_calendar(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!([{
            "title": "The Album",
            "releaseDate": "2020-01-06T00:00:00Z",
            "genres": ["Rock"],
            "statistics": {"percentOfTracks": 100.0},
            "artist": {
                "artistName": "The Band",
                "foreignArtistId": "abc-123",
                "ratings": {"value": 9},
                "images": [{"coverType": "fanart", "url": "http://art.example.net/band.jpg"}],
            },
        }]),
    )
}

/// A stand-in for Lidarr: one album in the calendar.
pub async fn lidarr() -> StandIn {
    StandIn::start(Router::new().route("/api/v1/calendar", get(lidarr_calendar))).await
}

/// The API key of the SABnzbd stand-in.
pub const SABNZBD_KEY: &str = "made-up-sabnzbd-key";

fn query_field<'a>(query: Option<&'a str>, name: &str) -> Option<&'a str> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

async fn sabnzbd_api(RawQuery(query): RawQuery) -> Response {
    let query = query.as_deref();
    if query_field(query, "apikey") != Some(SABNZBD_KEY) {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(json!({"status": false, "error": "API Key Incorrect"})),
        )
            .into_response();
    }
    let body = match (query_field(query, "mode"), query_field(query, "name")) {
        (Some("queue"), None) => json!({"queue": {
            "paused": false,
            "speed": "1.2 M",
            "timeleft": "0:10:00",
            "slots": [{
                "nzo_id": "SABnzbd_nzo_one",
                "filename": "The.Show.S02E03.1080p",
                "status": "Downloading",
                "percentage": "40",
                "timeleft": "0:10:00",
                "mb": "1000.00",
                "mbleft": "600.00",
                "cat": "tv",
            }],
        }}),
        (Some("history"), None) => json!({"history": {
            "slots": [{
                "nzo_id": "SABnzbd_nzo_old",
                "name": "The.Film.2020.1080p",
                "status": "Completed",
                "size": "4.1 GB",
                "category": "movies",
                "completed": 1_577_900_000,
            }],
        }}),
        (Some("pause" | "resume"), None) | (Some("queue"), Some("pause" | "resume")) => {
            json!({"status": true})
        }
        _ => json!({"status": false, "error": "not implemented"}),
    };
    axum::Json(body).into_response()
}

/// A stand-in for SABnzbd: one download in the queue and one in the history.
pub async fn sabnzbd() -> StandIn {
    StandIn::start(Router::new().route("/api", get(sabnzbd_api))).await
}

/// The user name and the password of the qBittorrent stand-in.
pub const QBITTORRENT_LOGIN: (&str, &str) = ("admin", "the password of qBittorrent");

const QBITTORRENT_SESSION: &str = "SID=made-up-qbittorrent-session";

async fn qbittorrent_login(body: axum::body::Bytes) -> Response {
    let body = String::from_utf8_lossy(&body).into_owned();
    // The password comes as a field of a form, with `+` for a space.
    let field = |name: &str| {
        query_field(Some(&body), name)
            .unwrap_or_default()
            .replace('+', " ")
    };
    if field("username") == QBITTORRENT_LOGIN.0 && field("password") == QBITTORRENT_LOGIN.1 {
        (
            [(
                header::SET_COOKIE,
                format!("{QBITTORRENT_SESSION}; HttpOnly; SameSite=Strict; path=/"),
            )],
            "Ok.",
        )
            .into_response()
    } else {
        "Fails.".into_response()
    }
}

async fn qbittorrent_torrents(headers: HeaderMap) -> Response {
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    if cookie != Some(QBITTORRENT_SESSION) {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    let torrent = |name: &str, state: &str, progress: f64| {
        json!({
            "name": name,
            "state": state,
            "progress": progress,
            "eta": 600,
            "size": 1_000_000,
            "dlspeed": 1000,
            "upspeed": 10,
            "category": "tv",
        })
    };
    axum::Json(json!([
        torrent("The.Show.S02E03", "downloading", 0.4),
        torrent("An.Old.Film", "stalledUP", 1.0),
        torrent("A.Finished.Film", "pausedUP", 1.0),
    ]))
    .into_response()
}

/// A stand-in for qBittorrent with the second version of its API: one torrent that
/// downloads, one that uploads, and one that is complete and paused.
pub async fn qbittorrent() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v2/auth/login", axum::routing::post(qbittorrent_login))
            .route("/api/v2/torrents/info", get(qbittorrent_torrents)),
    )
    .await
}

/// The API key of the BreezoMeter stand-in.
pub const WEATHER_KEY: &str = "made-up-weather-key";

/// The answer of the BreezoMeter stand-in: the given data for the right key, and else
/// the error that the API names in its answer.
fn for_weather_key(query: Option<&str>, data: &Value) -> Response {
    if query_field(query, "key") == Some(WEATHER_KEY) {
        axum::Json(json!({"metadata": null, "data": data, "error": null})).into_response()
    } else {
        (
            StatusCode::FORBIDDEN,
            axum::Json(json!({
                "metadata": null,
                "data": null,
                "error": {"code": "invalid_api_key", "title": "Invalid API key"},
            })),
        )
            .into_response()
    }
}

async fn weather_forecast(RawQuery(query): RawQuery) -> Response {
    let hour = |time: &str, text: &str, degrees: f64| {
        json!({
            "datetime": time,
            "is_day_time": true,
            "icon_code": 2,
            "weather_text": text,
            "temperature": {"value": degrees, "units": "C"},
            "feels_like_temperature": {"value": degrees - 1.0, "units": "C"},
            "relative_humidity": 61,
            "precipitation": {
                "precipitation_probability": 20,
                "total_precipitation": {"value": 0.0, "units": "mm"},
            },
            "wind": {"speed": {"value": 12.5, "units": "km/h"}, "direction": 270},
        })
    };
    for_weather_key(
        query.as_deref(),
        &json!([
            hour("2020-01-05T12:00:00Z", "Partly cloudy", 7.5),
            hour("2020-01-05T13:00:00Z", "Cloudy", 8.0),
        ]),
    )
}

async fn weather_air(RawQuery(query): RawQuery) -> Response {
    for_weather_key(
        query.as_deref(),
        &json!({
            "datetime": "2020-01-05T12:00:00Z",
            "data_available": true,
            "indexes": {"baqi": {
                "display_name": "BreezoMeter AQI",
                "aqi": 67,
                "aqi_display": "67",
                "color": "#84CF33",
                "category": "Good air quality",
                "dominant_pollutant": "pm25",
            }},
            "health_recommendations": {
                "general_population": "With this level of air quality, you have no limitations.",
            },
        }),
    )
}

async fn weather_pollen(RawQuery(query): RawQuery) -> Response {
    let kind = |name: &str, value: u8, category: &str| {
        json!({
            "display_name": name,
            "in_season": true,
            "data_available": true,
            "index": {"value": value, "category": category, "color": "#FFD900"},
        })
    };
    for_weather_key(
        query.as_deref(),
        &json!([{
            "date": "2020-01-05",
            "index_id": "bpi",
            "index_display_name": "BreezoMeter Pollen Index",
            "types": {
                "grass": kind("Grass", 1, "Very Low"),
                "tree": kind("Tree", 3, "Moderate"),
                "weed": {"display_name": "Weed", "in_season": false, "data_available": false, "index": {"value": null, "category": null, "color": null}},
            },
        }]),
    )
}

/// A stand-in for the API of BreezoMeter.
pub async fn breezometer() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/weather/v1/forecast/hourly", get(weather_forecast))
            .route("/air-quality/v2/current-conditions", get(weather_air))
            .route("/pollen/v2/forecast/daily", get(weather_pollen)),
    )
    .await
}

/// The API key of the stand-in for the statistics of Tautulli.
pub const TAUTULLI_KEY: &str = "made-up-tautulli-key";

async fn tautulli_api(RawQuery(query): RawQuery) -> Response {
    let query = query.as_deref();
    if query_field(query, "apikey") != Some(TAUTULLI_KEY) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(
                json!({"response": {"result": "error", "message": "Invalid apikey", "data": {}}}),
            ),
        )
            .into_response();
    }
    let data = match query_field(query, "cmd") {
        Some("get_home_stats") => json!([
            {"stat_id": "top_movies", "stat_title": "Most Watched Movies", "rows": [
                {"title": "The Film", "total_plays": 12, "section_id": 1, "rating_key": 815,
                 "art": "/library/metadata/815/art/1", "thumb": "/library/metadata/815/thumb/1"},
                {"title": "A Film For Children", "total_plays": 9, "section_id": 3, "rating_key": 816,
                 "art": "/library/metadata/816/art/1", "thumb": "/library/metadata/816/thumb/1"},
            ]},
            {"stat_id": "popular_tv", "stat_title": "Most Popular TV Shows", "rows": [
                {"title": "The Show", "users_watched": 4, "section_id": 2, "rating_key": 4711,
                 "art": "/library/metadata/4711/art/1", "thumb": "/library/metadata/4711/thumb/1"},
            ]},
            {"stat_id": "top_users", "stat_title": "Most Active Users", "rows": [
                {"user": "robin", "friendly_name": "Robin", "total_plays": 30},
            ]},
            {"stat_id": "top_platforms", "stat_title": "Most Active Platforms", "rows": [
                {"platform": "Chrome", "platform_name": "chrome", "total_plays": 20},
            ]},
            {"stat_id": "last_watched", "stat_title": "Recently Watched", "rows": [
                {"title": "The Film", "user": "robin"},
            ]},
            {"stat_id": "most_concurrent", "stat_title": "Most Concurrent Streams", "rows": []},
        ]),
        Some("get_libraries_table") => json!({
            "recordsTotal": 2,
            "data": [
                {"section_id": 1, "section_name": "Films", "section_type": "movie", "count": 120, "plays": 300},
                {"section_id": 3, "section_name": "Films for children", "section_type": "movie", "count": 20, "plays": 40},
            ],
        }),
        Some("pms_image_proxy") => {
            return ([(header::CONTENT_TYPE, "image/jpeg")], JPEG).into_response();
        }
        _ => json!({}),
    };
    axum::Json(json!({"response": {"result": "success", "message": null, "data": data}}))
        .into_response()
}

/// A stand-in for the API of Tautulli that the homepage reads.
pub async fn tautulli_statistics() -> StandIn {
    StandIn::start(Router::new().route("/api/v2", get(tautulli_api))).await
}
