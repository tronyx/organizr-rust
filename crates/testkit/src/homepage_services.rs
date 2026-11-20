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

/// The folders that an application stores its media in.
async fn arr_root_folders(headers: HeaderMap) -> Response {
    for_arr_key(
        &headers,
        json!([{"id": 1, "path": "/media", "accessible": true}]),
    )
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
            .route("/api/v3/rootfolder", get(arr_root_folders))
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
            .route("/api/v3/queue", get(radarr_queue))
            .route("/api/v3/rootfolder", get(arr_root_folders)),
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
    StandIn::start(
        Router::new()
            .route("/api/v1/calendar", get(lidarr_calendar))
            .route("/api/v1/rootfolder", get(arr_root_folders)),
    )
    .await
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

/// The account of the NZBGet stand-in: the name, the password, and the header that a
/// request with both carries.
pub const NZBGET_LOGIN: (&str, &str, &str) = (
    "nzbget",
    "the password of NZBGet",
    "Basic bnpiZ2V0OnRoZSBwYXNzd29yZCBvZiBOWkJHZXQ=",
);

fn has_login(headers: &HeaderMap, wanted: &str) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        == Some(wanted)
}

async fn nzbget_groups(headers: HeaderMap) -> Response {
    if !has_login(&headers, NZBGET_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "Access denied").into_response();
    }
    axum::Json(json!({"version": "1.1", "result": [
        {"NZBID": 4, "NZBName": "The.Show.S02E03", "Status": "DOWNLOADING",
         "FileSizeMB": 1000, "RemainingSizeMB": 250, "Category": "tv"},
        {"NZBID": 5, "NZBName": "A.Waiting.Film", "Status": "QUEUED",
         "FileSizeMB": 4000, "RemainingSizeMB": 4000, "Category": "movies"},
    ]}))
    .into_response()
}

async fn nzbget_history(headers: HeaderMap) -> Response {
    if !has_login(&headers, NZBGET_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "Access denied").into_response();
    }
    axum::Json(json!({"version": "1.1", "result": [
        {"NZBID": 3, "Name": "An.Old.Film", "Status": "SUCCESS/ALL", "FileSizeMB": 2000},
        {"NZBID": 2, "Name": "A.Broken.Film", "Status": "FAILURE/PAR", "FileSizeMB": 1500},
    ]}))
    .into_response()
}

/// A stand-in for NZBGet: two downloads in the queue and two in the history.
pub async fn nzbget() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/jsonrpc/listgroups", get(nzbget_groups))
            .route("/jsonrpc/history", get(nzbget_history)),
    )
    .await
}

/// The account of the Transmission stand-in: the name, the password, and the header
/// that a request with both carries.
pub const TRANSMISSION_LOGIN: (&str, &str, &str) = (
    "transmission",
    "the password of Transmission",
    "Basic dHJhbnNtaXNzaW9uOnRoZSBwYXNzd29yZCBvZiBUcmFuc21pc3Npb24=",
);

const TRANSMISSION_SESSION: &str = "made-up-transmission-session";

/// Transmission refuses a request without its session, and names the session.
async fn transmission_rpc(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    if !has_login(&headers, TRANSMISSION_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized User").into_response();
    }
    let session = headers
        .get("x-transmission-session-id")
        .and_then(|value| value.to_str().ok());
    if session != Some(TRANSMISSION_SESSION) {
        return (
            StatusCode::CONFLICT,
            [("X-Transmission-Session-Id", TRANSMISSION_SESSION)],
            "409: Conflict",
        )
            .into_response();
    }
    let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    if asked.get("method").and_then(Value::as_str) != Some("torrent-get") {
        return axum::Json(json!({"result": "method name not recognized"})).into_response();
    }
    let torrent = |id: i64, name: &str, status: i64, done: f64, added: i64| {
        json!({
            "id": id, "name": name, "status": status, "percentDone": done, "eta": 600,
            "totalSize": 1_000_000, "rateDownload": 1000, "isFinished": false,
            "isStalled": false, "downloadDir": "/downloads", "errorString": "",
            "addedDate": added,
        })
    };
    axum::Json(json!({"result": "success", "arguments": {"torrents": [
        torrent(3, "The.Show.S02E03", 4, 0.4, 300),
        torrent(1, "An.Old.Film", 6, 1.0, 100),
        torrent(2, "A.Stopped.Film", 0, 1.0, 200),
    ]}}))
    .into_response()
}

/// A stand-in for Transmission: one torrent that downloads, one that uploads, and one
/// that is stopped.
pub async fn transmission() -> StandIn {
    StandIn::start(Router::new().route("/rpc", get(transmission_rpc).post(transmission_rpc))).await
}

/// The token of the Pi-hole stand-in.
pub const PIHOLE_TOKEN: &str = "made-up-pihole-token";

/// Pi-hole 5 answers a request without the right token with an empty list.
async fn pihole_summary(RawQuery(query): RawQuery) -> Response {
    if query_field(query.as_deref(), "auth") != Some(PIHOLE_TOKEN) {
        return axum::Json(json!([])).into_response();
    }
    axum::Json(json!({
        "domains_being_blocked": 150_000,
        "dns_queries_today": 12_000,
        "ads_blocked_today": 3_000,
        "ads_percentage_today": 25.0,
        "unique_clients": 14,
        "status": "enabled",
    }))
    .into_response()
}

/// A stand-in for Pi-hole 5, with its web interface under `/admin`.
pub async fn pihole() -> StandIn {
    StandIn::start(Router::new().route("/admin/api.php", get(pihole_summary))).await
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
        Some("get_users") => json!([
            {"user_id": 0, "username": "Local", "friendly_name": "Local"},
            {"user_id": 11, "username": "robin", "friendly_name": "Robin R."},
        ]),
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

/// The token of the Plex Media Server stand-in.
pub const PLEX_SERVER_TOKEN: &str = "made-up-plex-server-token";

const PLEX_SESSIONS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<MediaContainer size="2">
  <Video type="episode" ratingKey="301" parentRatingKey="300" grandparentRatingKey="30" title="The One" parentTitle="Season 2" grandparentTitle="The Show" index="3" parentIndex="2" parentThumb="/library/metadata/300/thumb/1" grandparentArt="/library/metadata/30/art/1" librarySectionID="2" viewOffset="600000" duration="2400000" addedAt="1577880000">
    <Media videoResolution="1080"><Part decision="directplay"/></Media>
    <User id="1" title="robin" thumb="https://plex.tv/users/abc/avatar"/>
    <Player address="192.0.2.50" machineIdentifier="player-one" platform="Chrome" product="Plex Web" device="Windows" state="playing"/>
    <Session id="session-one" bandwidth="4000" location="lan"/>
  </Video>
  <Video type="movie" ratingKey="816" title="A Film For Children" year="2018" thumb="/library/metadata/816/thumb/1" art="/library/metadata/816/art/1" librarySectionID="3" duration="5000000">
    <User id="2" title="pat"/>
    <Player address="198.51.100.7" machineIdentifier="player-two" state="paused"/>
    <Session id="session-two" bandwidth="2000" location="wan"/>
  </Video>
</MediaContainer>"#;

fn plex_recent(kind: Option<&str>) -> &'static str {
    match kind {
        Some("1") => {
            r#"<MediaContainer size="1"><Video type="movie" ratingKey="815" title="The Film" year="2019" thumb="/library/metadata/815/thumb/1" art="/library/metadata/815/art/1" librarySectionID="1" addedAt="1577880000" duration="6000000"/></MediaContainer>"#
        }
        Some("2") => {
            r#"<MediaContainer size="1"><Directory type="season" ratingKey="300" parentRatingKey="30" title="Season 2" parentTitle="The Show" thumb="/library/metadata/300/thumb/1" art="/library/metadata/30/art/1" librarySectionID="2" addedAt="1577890000"/></MediaContainer>"#
        }
        _ => r#"<MediaContainer size="0"></MediaContainer>"#,
    }
}

const PLEX_PLAYLISTS: &str = r#"<MediaContainer size="4">
  <Playlist title="Film night!" playlistType="video" key="/playlists/7/items"/>
  <Playlist title="A private list" playlistType="video" key="/playlists/8/items"/>
  <Playlist title="Elsewhere" playlistType="video" key="/status/sessions"/>
  <Playlist title="Songs" playlistType="audio" key="/playlists/9/items"/>
</MediaContainer>"#;

const PLEX_FILM: &str = r#"<MediaContainer size="1"><Video type="movie" ratingKey="815" title="The Film" year="2019" summary="A film." thumb="/library/metadata/815/thumb/1" art="/library/metadata/815/art/1" librarySectionID="1" duration="6000000"><Genre tag="Drama"/></Video></MediaContainer>"#;

const PLEX_FOUND: &str = r#"<MediaContainer size="2">
  <Video type="movie" ratingKey="815" title="The Film" librarySectionID="1"/>
  <Video type="episode" ratingKey="301" title="The One" librarySectionID="2"/>
</MediaContainer>"#;

async fn plex_server_api(
    headers: HeaderMap,
    uri: axum::http::Uri,
    RawQuery(query): RawQuery,
) -> Response {
    // The token comes in a header, or in the query as the connection test sends it.
    let token = headers
        .get("x-plex-token")
        .and_then(|value| value.to_str().ok())
        .or_else(|| query_field(query.as_deref(), "X-Plex-Token"));
    if token != Some(PLEX_SERVER_TOKEN) {
        return (
            StatusCode::UNAUTHORIZED,
            "<html><body>401 Unauthorized</body></html>",
        )
            .into_response();
    }
    let xml = |body: &'static str| {
        ([(header::CONTENT_TYPE, "text/xml;charset=utf-8")], body).into_response()
    };
    match uri.path() {
        "/status/sessions" => xml(PLEX_SESSIONS),
        "/servers" => xml("<MediaContainer size=\"1\"><Server name=\"Media\"/></MediaContainer>"),
        "/hubs/home/recentlyAdded" => xml(plex_recent(query_field(query.as_deref(), "type"))),
        "/playlists" => xml(PLEX_PLAYLISTS),
        "/playlists/7/items" | "/library/metadata/815" => xml(PLEX_FILM),
        "/search" => xml(PLEX_FOUND),
        "/photo/:/transcode" => ([(header::CONTENT_TYPE, "image/jpeg")], JPEG).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

/// A stand-in for a Plex Media Server: two sessions, two items that were added, four
/// playlists, one film to look at and to find, and pictures.
pub async fn plex_server() -> StandIn {
    StandIn::start(Router::new().fallback(plex_server_api)).await
}

/// The API key of the stand-ins for an Emby server and a Jellyfin server.
pub const MEDIA_SERVER_KEY: &str = "made-up-media-server-key";

fn media_film() -> Value {
    json!({
        "Id": "815", "ServerId": "made-up-server-id", "Type": "Movie", "Name": "The Film",
        "ProductionYear": 2019, "RunTimeTicks": 60_000_000_000_i64, "CommunityRating": 7.5,
        "PremiereDate": "2019-05-01T00:00:00.0000000Z", "Overview": "A film.",
        "Taglines": ["One film."], "Genres": ["Drama"],
        "ImageTags": {"Primary": "made-up-tag"}, "BackdropImageTags": ["made-up-tag"],
        "People": [
            {"Id": "9", "Name": "An Actor", "Role": "The Hero", "Type": "Actor", "PrimaryImageTag": "made-up-tag"},
            {"Id": "10", "Name": "A Director", "Role": "", "Type": "Director"},
        ],
    })
}

fn media_sessions() -> Value {
    json!([
        {
            "Id": "session-one", "UserName": "robin", "Client": "Web", "DeviceName": "Chrome",
            "DeviceId": "device-one", "RemoteEndPoint": "192.0.2.50",
            "PlayState": {"PositionTicks": 6_000_000_000_i64, "IsPaused": true, "PlayMethod": "Transcode"},
            "TranscodingInfo": {"CompletionPercentage": 40.5, "Bitrate": 4_000_000, "VideoCodec": "h264", "AudioCodec": "aac", "AudioChannels": 2},
            "NowPlayingItem": {
                "Id": "301", "ServerId": "made-up-server-id", "Type": "Episode", "Name": "The One",
                "SeriesName": "The Show", "SeriesId": "30", "ParentIndexNumber": 2, "IndexNumber": 3,
                "ParentBackdropItemId": "30", "ParentBackdropImageTags": ["made-up-tag"],
                "ImageTags": {"Primary": "made-up-tag"}, "RunTimeTicks": 24_000_000_000_i64,
                "Container": "mkv", "MediaStreams": [{"Codec": "hevc", "Width": 1920}, {"Codec": "eac3"}],
            },
        },
        // A player that plays nothing.
        {"Id": "session-two", "UserName": "pat", "Client": "Television", "DeviceName": "Living room", "DeviceId": "device-two", "PlayState": {"IsPaused": false}},
        // A type that the homepage does not show.
        {
            "Id": "session-three", "UserName": "pat", "Client": "Television", "DeviceName": "Kitchen", "DeviceId": "device-three",
            "PlayState": {"IsPaused": false, "PlayMethod": "DirectPlay"},
            "NowPlayingItem": {"Id": "77", "ServerId": "made-up-server-id", "Type": "TvChannel", "Name": "A Channel"},
        },
        {
            "Id": "session-four", "UserName": "pat", "Client": "Phone", "DeviceName": "A phone", "DeviceId": "device-four",
            "RemoteEndPoint": "198.51.100.7",
            "PlayState": {"PositionTicks": 0, "IsPaused": false, "PlayMethod": "DirectPlay"},
            "NowPlayingItem": media_film(),
        },
    ])
}

/// What was added, as the user with the given id sees it: the user 2 played the film.
fn media_latest(user: &str) -> Value {
    let series = json!({
        "Id": "30", "ServerId": "made-up-server-id", "Type": "Series", "Name": "The Show",
        "ImageTags": {"Primary": "made-up-tag", "Thumb": "made-up-tag"},
    });
    if user == "2" {
        json!([series])
    } else {
        json!([media_film(), series])
    }
}

fn media_server_api(
    key_header: (&str, &str),
    headers: &HeaderMap,
    uri: &axum::http::Uri,
) -> Response {
    let path = uri.path();
    // The servers give their pictures without a key.
    if path.starts_with("/Items/") && path.contains("/Images/") {
        return ([(header::CONTENT_TYPE, "image/jpeg")], JPEG).into_response();
    }
    let sent = headers
        .get(key_header.0)
        .and_then(|value| value.to_str().ok());
    if sent != Some(key_header.1) {
        return (
            StatusCode::UNAUTHORIZED,
            "Access token is invalid or expired.",
        )
            .into_response();
    }
    let body = match path {
        "/Sessions" => media_sessions(),
        "/Users" => json!([
            {"Id": "1", "Name": "Owner", "Policy": {"IsAdministrator": true}},
            {"Id": "2", "Name": "Zed", "Policy": {"IsAdministrator": false}},
        ]),
        "/Users/1/Items/Latest" => media_latest("1"),
        "/Users/2/Items/Latest" => media_latest("2"),
        "/Users/1/Items/815" | "/Users/2/Items/815" => media_film(),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    axum::Json(body).into_response()
}

fn media_server(header: &'static str, value: String) -> Router {
    Router::new().fallback(move |headers: HeaderMap, uri: axum::http::Uri| {
        std::future::ready(media_server_api((header, &value), &headers, &uri))
    })
}

/// A stand-in for an Emby server: four sessions, two users, two items that were
/// added, one film to look at, and pictures. The key comes in the header
/// `X-Emby-Token`.
pub async fn emby() -> StandIn {
    StandIn::start(media_server("x-emby-token", MEDIA_SERVER_KEY.to_owned())).await
}

/// A stand-in for a Jellyfin server, with what the stand-in for Emby has. The key
/// comes in the header `Authorization`.
pub async fn jellyfin() -> StandIn {
    let value = format!("MediaBrowser Token=\"{MEDIA_SERVER_KEY}\"");
    StandIn::start(media_server("authorization", value)).await
}

/// The API key of the stand-in for the requests of Overseerr.
pub const OVERSEERR_KEY: &str = "made-up-overseerr-key";

/// The session of the user "robin" at the stand-in for the requests of Overseerr, as
/// the browser holds it in the `connect.sid` cookie.
pub const OVERSEERR_SESSION: &str = "s%3Amade-up-session-of-robin.signature";

fn overseerr_request(
    id: u32,
    kind: &str,
    tmdb: u32,
    user: &str,
    status: u8,
    media_status: u8,
    created: &str,
) -> Value {
    json!({
        "id": id,
        "status": status,
        "type": kind,
        "createdAt": created,
        "media": {"id": id + 40, "tmdbId": tmdb, "status": media_status},
        "requestedBy": {"id": 1, "username": null, "plexUsername": user, "displayName": user, "email": format!("{user}@example.com")},
    })
}

fn overseerr_reply(
    method: &str,
    path: &str,
    by_key: bool,
    by_user: bool,
    body: &Value,
) -> (StatusCode, Value) {
    let done = (StatusCode::OK, json!({"ok": true}));
    match (method, path) {
        ("GET", "/api/v1/request" | "/api/v1/request/") if by_key => (
            StatusCode::OK,
            json!({"pageInfo": {"results": 3}, "results": [
                overseerr_request(31, "movie", 815, "robin", 2, 5, "2020-01-03T10:00:00.000Z"),
                overseerr_request(32, "tv", 4711, "pat", 1, 3, "2020-01-05T10:00:00.000Z"),
                overseerr_request(33, "movie", 999, "pat", 3, 1, "2020-01-04T10:00:00.000Z"),
            ]}),
        ),
        ("GET", "/api/v1/movie/815") if by_key => (
            StatusCode::OK,
            json!({"id": 815, "title": "The Film", "releaseDate": "2019-05-01", "overview": "A film.", "posterPath": "/poster815.jpg", "backdropPath": "/back815.jpg"}),
        ),
        ("GET", "/api/v1/tv/4711") if by_key => (
            StatusCode::OK,
            json!({
                "id": 4711, "name": "The Show", "firstAirDate": "2018-09-01", "overview": "A show.", "posterPath": "", "backdropPath": null,
                "externalIds": {"tvdbId": 1234},
                "seasons": [{"seasonNumber": 0}, {"seasonNumber": 1}, {"seasonNumber": 2}],
            }),
        ),
        ("GET", "/api/v1/request/31") if by_key => {
            (StatusCode::OK, json!({"id": 31, "media": {"id": 71}}))
        }
        (
            "POST",
            "/api/v1/request/31/approve"
            | "/api/v1/request/31/pending"
            | "/api/v1/request/31/decline"
            | "/api/v1/media/71/available"
            | "/api/v1/media/71/pending",
        ) if by_key => done,
        ("DELETE", "/api/v1/request/31") if by_key => (StatusCode::NO_CONTENT, Value::Null),
        ("POST", "/api/v1/request/77/approve") if by_key => (
            StatusCode::NOT_FOUND,
            json!({"message": "Request not found."}),
        ),
        ("GET", "/api/v1/auth/me") if by_user => {
            (StatusCode::OK, json!({"id": 7, "plexUsername": "robin"}))
        }
        ("GET", "/api/v1/service/sonarr") if by_key => (
            StatusCode::OK,
            json!([{"id": 0, "name": "Sonarr", "is4k": false, "isDefault": true, "activeProfileId": 4, "activeDirectory": "/tv", "activeLanguageProfileId": 1}]),
        ),
        ("GET", "/api/v1/service/radarr") if by_key => (
            StatusCode::OK,
            json!([
                {"id": 0, "name": "Radarr", "is4k": false, "isDefault": true, "activeProfileId": 6},
                {"id": 1, "name": "Radarr 4K", "is4k": true, "isDefault": true, "activeProfileId": 9},
            ]),
        ),
        ("POST", "/api/v1/request") if by_user => {
            if body["mediaId"] == 500 {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({"message": "No seasons available to request"}),
                )
            } else {
                (StatusCode::CREATED, json!({"id": 40}))
            }
        }
        _ if !by_key && !by_user => (
            StatusCode::FORBIDDEN,
            json!({"message": "You do not have permission to access this endpoint"}),
        ),
        ("GET", "/api/v1/settings/main") if by_key => {
            (StatusCode::OK, json!({"applicationTitle": "Overseerr"}))
        }
        ("GET", "/api/v1/settings/main") => (
            StatusCode::FORBIDDEN,
            json!({"message": "You do not have permission to access this endpoint"}),
        ),
        _ => (StatusCode::NOT_FOUND, json!({"message": "Not found"})),
    }
}

async fn overseerr_api(
    headers: HeaderMap,
    method: axum::http::Method,
    uri: axum::http::Uri,
    body: axum::body::Bytes,
) -> Response {
    let header_text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let by_key = header_text("x-api-key") == Some(OVERSEERR_KEY);
    let session = format!("connect.sid={OVERSEERR_SESSION}");
    let by_user = header_text("cookie") == Some(session.as_str());
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let (status, answer) = overseerr_reply(method.as_str(), uri.path(), by_key, by_user, &body);
    if answer.is_null() {
        status.into_response()
    } else {
        (status, axum::Json(answer)).into_response()
    }
}

/// A stand-in for the API of Overseerr that the homepage uses: three requests, of
/// which one names a film that Overseerr has no details for.
pub async fn overseerr_requests() -> StandIn {
    StandIn::start(Router::new().fallback(overseerr_api)).await
}
