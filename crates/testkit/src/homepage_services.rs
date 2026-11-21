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

/// The password of the web interface of the Deluge stand-in.
pub const DELUGE_PASSWORD: &str = "the password of Deluge";

const DELUGE_SESSION: &str = "_session_id=made-up-deluge-session";

/// The JSON-RPC API of the web interface of Deluge, with the `WebAPI` plugin.
async fn deluge_json(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let id = asked.get("id").cloned().unwrap_or(Value::Null);
    let answer = |result: Value, error: Value| {
        axum::Json(json!({"id": id, "result": result, "error": error})).into_response()
    };
    let has_session = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        == Some(DELUGE_SESSION);
    match asked.get("method").and_then(Value::as_str) {
        Some("auth.login") => {
            if asked.pointer("/params/0").and_then(Value::as_str) == Some(DELUGE_PASSWORD) {
                (
                    [(
                        header::SET_COOKIE,
                        format!(
                            "{DELUGE_SESSION}; Expires=Fri, 01 Jan 2100 00:00:00 GMT; Path=/json"
                        ),
                    )],
                    answer(json!(true), Value::Null),
                )
                    .into_response()
            } else {
                answer(json!(false), Value::Null)
            }
        }
        Some(_) if !has_session => answer(
            Value::Null,
            json!({"message": "Not authenticated", "code": 1}),
        ),
        Some("webapi.get_torrents") => {
            let torrent = |name: &str, queue: i64, progress: f64, state: &str| {
                json!({
                    "comment": "", "download_payload_rate": 1000, "eta": 600,
                    "hash": format!("made-up-hash-of-{name}"), "is_finished": progress >= 100.0,
                    "is_seed": progress >= 100.0, "message": "OK", "name": name,
                    "paused": state == "Paused", "progress": progress, "queue": queue,
                    "state": state, "total_size": 1_000_000, "upload_payload_rate": 10,
                    "tracker_status": "Announce OK",
                })
            };
            answer(
                json!({"torrents": [
                    torrent("The.Show.S02E03", 0, 40.5, "Downloading"),
                    torrent("An.Old.Film", -1, 100.0, "Seeding"),
                    torrent("A.Paused.Film", -1, 100.0, "Paused"),
                ]}),
                Value::Null,
            )
        }
        _ => answer(Value::Null, json!({"message": "Unknown method", "code": 2})),
    }
}

/// A stand-in for the web interface of Deluge with the `WebAPI` plugin: one torrent
/// that downloads, one that uploads, and one that is complete and paused.
pub async fn deluge() -> StandIn {
    StandIn::start(Router::new().route("/json", axum::routing::post(deluge_json))).await
}

/// The account of the rTorrent stand-in: the name, the password, and the header that a
/// request with both carries.
pub const RTORRENT_LOGIN: (&str, &str, &str) = (
    "rtorrent",
    "the password of rTorrent",
    "Basic cnRvcnJlbnQ6dGhlIHBhc3N3b3JkIG9mIHJUb3JyZW50",
);

/// One torrent of the rTorrent stand-in.
struct RtorrentRow {
    name: &'static str,
    connection: &'static str,
    started: i64,
    open: i64,
    complete: i64,
    label: &'static str,
}

impl RtorrentRow {
    /// The values of the torrent, in the order that Organizr asks for them. rTorrent
    /// sends each number as `i8`.
    fn xml(&self) -> String {
        let text = |text: &str| format!("<value><string>{text}</string></value>");
        let number = |number: i64| format!("<value><i8>{number}</i8></value>");
        let done = if self.complete == 1 {
            1_000_000
        } else {
            400_000
        };
        let values = [
            text(self.name),
            text(&format!("/downloads/{}", self.name)),
            number(5000),
            number(1_000_000),
            number(done),
            number(done),
            text(self.connection),
            number(1000),
            number(10),
            number(self.started),
            number(self.open),
            text(""),
            text(&format!("HASH{}", self.started)),
            number(self.complete),
            number(5),
            number(262_144),
            number(0),
            number(0),
            number(0),
            text(""),
            text(self.label),
            text(""),
            text(""),
            text(""),
            text(""),
        ];
        format!(
            "<value><array><data>{}</data></array></value>",
            values.concat()
        )
    }
}

const RTORRENT_ROWS: [RtorrentRow; 3] = [
    RtorrentRow {
        name: "The.Show.S02E03",
        connection: "leech",
        started: 300,
        open: 1,
        complete: 0,
        label: "tv",
    },
    RtorrentRow {
        name: "An.Old.Film",
        connection: "seed",
        started: 100,
        open: 1,
        complete: 1,
        label: "films",
    },
    RtorrentRow {
        name: "A.Closed.Film",
        connection: "seed",
        started: 200,
        open: 0,
        complete: 1,
        label: "",
    },
];

/// What rTorrent answers to a method of XML-RPC.
fn rtorrent_answer(body: &str) -> Response {
    let list = |values: String| {
        (
            [(header::CONTENT_TYPE, "text/xml")],
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><methodResponse><params><param>\
                 <value><array><data>{values}</data></array></value>\
                 </param></params></methodResponse>"
            ),
        )
            .into_response()
    };
    if body.contains("<methodName>system.listMethods</methodName>") {
        list(
            "<value><string>system.listMethods</string></value>\
             <value><string>d.multicall2</string></value>"
                .to_owned(),
        )
    } else if body.contains("<methodName>d.multicall2</methodName>") {
        list(RTORRENT_ROWS.iter().map(RtorrentRow::xml).collect())
    } else {
        (
            [(header::CONTENT_TYPE, "text/xml")],
            "<?xml version=\"1.0\"?><methodResponse><fault><value><struct>\
             <member><name>faultCode</name><value><i4>-506</i4></value></member>\
             <member><name>faultString</name><value><string>Method not defined</string></value>\
             </member></struct></value></fault></methodResponse>",
        )
            .into_response()
    }
}

/// rTorrent behind a web server that asks for the name and the password with Basic.
async fn rtorrent_rpc(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    if !has_login(&headers, RTORRENT_LOGIN.2) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"rTorrent\"")],
            "Unauthorized",
        )
            .into_response();
    }
    rtorrent_answer(&String::from_utf8_lossy(&body))
}

/// rTorrent behind a web server that asks with Digest. The stand-in looks at the shape
/// of the answer to its challenge, and not at the hash.
async fn rtorrent_rpc_with_digest(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    let answer = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let answered = answer.starts_with("Digest ")
        && answer.contains(&format!("username=\"{}\"", RTORRENT_LOGIN.0))
        && answer.contains("nonce=\"made-up-nonce\"")
        && answer.contains("uri=\"/digest/RPC2\"")
        && answer.contains("response=\"");
    if !answered {
        return (
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                "Digest realm=\"rTorrent\", qop=\"auth\", nonce=\"made-up-nonce\"",
            )],
            "Unauthorized",
        )
            .into_response();
    }
    rtorrent_answer(&String::from_utf8_lossy(&body))
}

/// A stand-in for rTorrent: one torrent that downloads, one that uploads, and one that
/// is complete and closed. `/RPC2` asks for the login with Basic, `/digest/RPC2` asks
/// with Digest, and `/no-list/RPC2` answers every method with a fault.
pub async fn rtorrent() -> StandIn {
    use axum::routing::post;
    StandIn::start(
        Router::new()
            .route("/RPC2", post(rtorrent_rpc))
            .route("/digest/RPC2", post(rtorrent_rpc_with_digest))
            .route("/no-list/RPC2", post(|| async { rtorrent_answer("") })),
    )
    .await
}

/// The account of the uTorrent stand-in: the name, the password, and the header that a
/// request with both carries.
pub const UTORRENT_LOGIN: (&str, &str, &str) = (
    "utorrent",
    "the password of uTorrent",
    "Basic dXRvcnJlbnQ6dGhlIHBhc3N3b3JkIG9mIHVUb3JyZW50",
);

/// The token of the uTorrent stand-in.
pub const UTORRENT_TOKEN: &str = "made-up_uTorrent-token";

const UTORRENT_COOKIE: &str = "GUID=made-up-guid";

async fn utorrent_token(headers: HeaderMap) -> Response {
    if !has_login(&headers, UTORRENT_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response();
    }
    (
        [(header::SET_COOKIE, format!("{UTORRENT_COOKIE}; path=/"))],
        format!("<html><div id='token' style='display:none;'>{UTORRENT_TOKEN}</div></html>"),
    )
        .into_response()
}

async fn utorrent_list(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    if !has_login(&headers, UTORRENT_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "401 Unauthorized").into_response();
    }
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    if query_field(query.as_deref(), "token") != Some(UTORRENT_TOKEN)
        || cookie != Some(UTORRENT_COOKIE)
    {
        return (StatusCode::BAD_REQUEST, "invalid request").into_response();
    }
    let torrent = |name: &str, status: i64, progress: i64, message: &str| {
        json!([
            format!("HASH-OF-{name}"),
            status,
            name,
            1_000_000,
            progress,
            400_000,
            5000,
            12,
            10,
            1000,
            600,
            "tv",
            2,
            5,
            1,
            3,
            65536,
            1,
            600_000,
            "",
            "",
            message,
            "1",
            1_600_000_000,
            0,
            "",
            "/downloads",
            0,
            "ABCD",
        ])
    };
    axum::Json(json!({
        "build": 45000,
        "label": [["tv", 3]],
        "torrents": [
            torrent("The.Show.S02E03", 201, 400, "Downloading 40.0 %"),
            torrent("An.Old.Film", 201, 1000, "Seeding 100.0 %"),
            torrent("A.Finished.Film", 136, 1000, "Finished 100.0 %"),
        ],
        "torrentc": "1234567890",
    }))
    .into_response()
}

/// A stand-in for the web interface of uTorrent: one torrent that downloads, one that
/// uploads, and one that is finished.
pub async fn utorrent() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/gui/token.html", get(utorrent_token).post(utorrent_token))
            .route("/gui/", get(utorrent_list)),
    )
    .await
}

/// The account of the stand-in for the service of JDownloader: the name, the password,
/// and the header that a request with both carries.
pub const JDOWNLOADER_LOGIN: (&str, &str, &str) = (
    "jdownloader",
    "the password of JDownloader",
    "Basic amRvd25sb2FkZXI6dGhlIHBhc3N3b3JkIG9mIEpEb3dubG9hZGVy",
);

async fn jdownloader_state(headers: HeaderMap) -> Response {
    if !has_login(&headers, JDOWNLOADER_LOGIN.2) {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }
    axum::Json(json!({
        "downloader_state": "RUNNING",
        "grabber_collecting": false,
        "update_ready": false,
        "packages": {
            "downloader": [{
                "name": "A.Package", "speed": "1.2 MB/s", "size": "4.00 GB", "done": "1.00 GB",
                "eta": "40m:00s", "percentage": "25", "enabled": true,
            }],
            "linkgrabber_decrypted": [{"name": "A.New.Package", "size": "700.00 MB"}],
            "linkgrabber_failed": [],
            "linkgrabber_offline": [{"name": "A.Lost.Package"}],
        },
    }))
    .into_response()
}

/// A stand-in for the service that reads JDownloader, as the Python module `myjd-api`
/// answers: one package that downloads, one that waits, and one that is offline.
pub async fn jdownloader() -> StandIn {
    StandIn::start(Router::new().route("/", get(jdownloader_state))).await
}

/// The API key of the stand-in for the requests of Ombi.
pub const OMBI_KEY: &str = "made-up-ombi-key";

/// The token of the user "robin" at the stand-in for the requests of Ombi, as the
/// browser holds it in the `Auth` cookie.
pub const OMBI_TOKEN: &str = "made-up-ombi-token-of-robin";

fn ombi_user(name: &str) -> Value {
    json!({"userName": name, "userAlias": format!("{name} the alias")})
}

fn ombi_films() -> Value {
    json!([
        {
            "id": 31, "theMovieDbId": 815, "title": "The Film", "overview": "A film.",
            "posterPath": "/poster815.jpg", "background": "/back815.jpg",
            "approved": true, "available": true, "denied": false, "deniedReason": null,
            "requestedUser": ombi_user("robin"),
            "requestedDate": "2020-01-03T10:00:00", "releaseDate": "2019-05-01T00:00:00",
        },
        {
            "id": 33, "theMovieDbId": 999, "title": "Denied Film", "overview": null,
            "posterPath": "", "background": null,
            "approved": false, "available": false, "denied": true, "deniedReason": "No",
            "requestedUser": ombi_user("pat"),
            "requestedDate": "2020-01-04T10:00:00", "releaseDate": "2019-06-01T00:00:00",
        },
    ])
}

fn ombi_series() -> Value {
    json!([
        {
            "id": 32, "tvDbId": 4711, "title": "The Show", "overview": "A show.",
            "posterPath": "https://art.example.net/show.jpg", "background": "",
            "releaseDate": "2018-09-01T00:00:00",
            "childRequests": [{
                "id": 52, "approved": false, "available": false, "denied": false, "deniedReason": null,
                "requestedUser": ombi_user("pat"), "requestedDate": "2020-01-05T10:00:00",
            }],
        },
        {"id": 34, "tvDbId": 4799, "title": "No Children", "childRequests": []},
    ])
}

fn ombi_reply(
    method: &str,
    path: &str,
    by_key: bool,
    by_user: bool,
    body: &Value,
) -> (StatusCode, Value) {
    let done = (StatusCode::OK, json!({"result": true}));
    let known = |available: bool, requested: bool| {
        (
            StatusCode::OK,
            json!({"theTvDbId": "1234", "available": available, "requested": requested}),
        )
    };
    match (method, path) {
        ("GET", "/api/v1/Request/movie") if by_key => (StatusCode::OK, ombi_films()),
        ("GET", "/api/v1/Request/tv") if by_key => (StatusCode::OK, ombi_series()),
        ("GET", "/api/v1/Settings/about") if by_key => {
            (StatusCode::OK, json!({"version": "4.0.0"}))
        }
        (
            "POST",
            "/api/v1/Request/movie/approve"
            | "/api/v1/Request/movie/available"
            | "/api/v1/Request/movie/unavailable"
            | "/api/v1/Request/tv/approve",
        )
        | ("PUT", "/api/v1/Request/movie/deny")
            if by_key && (body["id"] == 31 || body["id"] == 32) =>
        {
            done
        }
        ("DELETE", "/api/v1/Request/movie/31" | "/api/v1/Request/tv/32") if by_key => done,
        ("GET", "/api/v1/Search/movie/info/123" | "/api/v1/Search/movie/info/500") if by_user => {
            known(false, false)
        }
        ("GET", "/api/v1/Search/movie/info/815") if by_user => known(true, true),
        ("GET", "/api/v1/Search/movie/info/816") if by_user => known(false, true),
        ("GET", "/api/v2/Search/tv/moviedb/4712") if by_user => known(false, false),
        ("GET", "/api/v2/Search/tv/moviedb/4713") if by_user => (
            StatusCode::OK,
            json!({"theTvDbId": null, "available": false, "requested": false}),
        ),
        ("POST", "/api/v1/Request/movie") if by_user && body["theMovieDbId"] != 500 => done,
        ("POST", "/api/v1/Request/tv") if by_user => done,
        // Ombi answers a request without a key or a token with no text.
        _ if !by_key && !by_user => (StatusCode::UNAUTHORIZED, Value::Null),
        _ => (StatusCode::NOT_FOUND, json!({"message": "Not found"})),
    }
}

async fn ombi_api(
    headers: HeaderMap,
    method: axum::http::Method,
    uri: axum::http::Uri,
    body: axum::body::Bytes,
) -> Response {
    let header_text = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let by_key = header_text("apikey") == Some(OMBI_KEY);
    let token = format!("Bearer {OMBI_TOKEN}");
    let by_user = header_text("authorization") == Some(token.as_str());
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let (status, answer) = ombi_reply(method.as_str(), uri.path(), by_key, by_user, &body);
    if answer.is_null() {
        status.into_response()
    } else {
        (status, axum::Json(answer)).into_response()
    }
}

/// A stand-in for the API of Ombi that the homepage uses: two requests for films,
/// and two for series, of which one has no child request.
pub async fn ombi_requests() -> StandIn {
    StandIn::start(Router::new().fallback(ombi_api)).await
}

/// The API key of the stand-in for SickRage.
pub const SICKRAGE_KEY: &str = "made-up-sickrage-key";

fn sickrage_episode(day: &str, name: &str) -> Value {
    json!({
        "airdate": day, "airs": "Sunday 9:00 PM", "ep_name": name, "ep_plot": "What happens.",
        "episode": 3, "season": 2, "network": "The Network", "paused": 0, "quality": "HD720p",
        "show_name": "The Show", "show_status": "Continuing", "tvdbid": 4711, "weekday": 7,
    })
}

async fn sickrage_api(
    axum::extract::Path(key): axum::extract::Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    if key != SICKRAGE_KEY {
        // SickRage answers a wrong key with the status 200.
        return axum::Json(json!({"data": {}, "message": "WRONG API KEY", "result": "denied"}))
            .into_response();
    }
    let data = match query_field(query.as_deref(), "cmd") {
        Some("future") => json!({
            "missed": [sickrage_episode("2020-01-05", "The Missed One")],
            "today": [],
            "soon": [],
            "later": [sickrage_episode("2099-01-04", "The Late One")],
        }),
        Some("history") => json!([{
            "date": "2020-01-06 22:10", "episode": 2, "season": 2, "show_name": "The Show",
            "status": "Downloaded", "tvdbid": 4711, "quality": "HD720p", "provider": "-1",
            "resource": "The.Show.S02E02.720p.mkv", "resource_path": "/tv/The Show",
        }]),
        Some("sb") => json!({"api_version": 4, "sr_version": "10.0.0"}),
        _ => {
            return axum::Json(json!({"data": {}, "message": "No such cmd", "result": "error"}))
                .into_response();
        }
    };
    axum::Json(json!({"data": data, "message": "", "result": "success"})).into_response()
}

/// A stand-in for SickRage: one episode that was missed, one far in the future, and
/// one that was downloaded.
pub async fn sickrage() -> StandIn {
    StandIn::start(Router::new().route("/api/{key}/", get(sickrage_api))).await
}

/// The API key of the stand-in for CouchPotato.
pub const COUCHPOTATO_KEY: &str = "made-up-couchpotato-key";

/// A day as text, a number of days after today in UTC.
fn day_from_today(days: i64) -> String {
    let today = jiff::Timestamp::now()
        .to_zoned_datetime(jiff::tz::TimeZone::UTC)
        .date();
    today
        .checked_add(jiff::Span::new().days(days))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string()
}

fn couchpotato_film(title: &str, id: u32, status: &str, released: &str) -> Value {
    json!({
        "status": status,
        "info": {
            "original_title": title, "tmdb_id": id, "released": released,
            "tagline": "A tagline.", "plot": "A film.", "runtime": 101, "year": 2020,
            "genres": ["Drama"], "rating": {"imdb": [7.2, 1000]},
            "images": {"backdrop_original": [], "backdrop": []},
        },
        "releases": [],
    })
}

async fn couchpotato_films(
    axum::extract::Path(key): axum::extract::Path<String>,
    RawQuery(query): RawQuery,
) -> Response {
    if key != COUCHPOTATO_KEY || query.as_deref() != Some("status=active%2Cdone") {
        return StatusCode::NOT_FOUND.into_response();
    }
    axum::Json(json!({
        "success": true,
        "empty": false,
        "movies": [
            couchpotato_film("An Old Film", 814, "done", "2019-01-07"),
            couchpotato_film("The Film", 815, "done", &day_from_today(-2)),
            couchpotato_film("The Next Film", 816, "active", &day_from_today(5)),
        ],
    }))
    .into_response()
}

/// A stand-in for CouchPotato: one film from long ago, one of two days ago that it
/// has, and one that comes in five days.
pub async fn couchpotato() -> StandIn {
    StandIn::start(Router::new().route("/api/{key}/media.list", get(couchpotato_films))).await
}

async fn ics_file() -> Response {
    let day = |days: i64| day_from_today(days).replace('-', "");
    let text = [
        "BEGIN:VCALENDAR".to_owned(),
        "VERSION:2.0".to_owned(),
        "PRODID:-//made-up//calendar//EN".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        "UID:one@example.com".to_owned(),
        format!("DTSTART:{}T170000Z", day(1)),
        format!("DTEND:{}T183000Z", day(1)),
        "SUMMARY:Dinner".to_owned(),
        "END:VEVENT".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        "UID:two@example.com".to_owned(),
        format!("DTSTART;VALUE=DATE:{}", day(-3)),
        format!("DTEND;VALUE=DATE:{}", day(-2)),
        "SUMMARY:A birthday".to_owned(),
        "END:VEVENT".to_owned(),
        "BEGIN:VEVENT".to_owned(),
        "UID:three@example.com".to_owned(),
        "DTSTART:20190105T170000Z".to_owned(),
        "DTEND:20190105T180000Z".to_owned(),
        "SUMMARY:Long ago".to_owned(),
        "END:VEVENT".to_owned(),
        "END:VCALENDAR".to_owned(),
    ]
    .join("\r\n");
    (
        [(header::CONTENT_TYPE, "text/calendar; charset=utf-8")],
        text,
    )
        .into_response()
}

/// A stand-in for a server that hands out a calendar file at `/calendar.ics`: a
/// dinner tomorrow, a birthday three days ago, and an event of the year 2019.
pub async fn ics() -> StandIn {
    StandIn::start(Router::new().route("/calendar.ics", get(ics_file))).await
}

/// The API key of the Jackett stand-in.
pub const JACKETT_KEY: &str = "made-up-jackett-key";

/// Jackett answers a search with the releases of every indexer. Its download links
/// hold its API key.
async fn jackett_results(RawQuery(query): RawQuery) -> Response {
    let query = query.as_deref();
    if query_field(query, "apikey") != Some(JACKETT_KEY) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let link = |folder: &str| {
        format!(
            "http://127.0.0.1:9117/{folder}/tracker/?jackett_apikey={JACKETT_KEY}&path=made-up-path&file=A+Film+2024"
        )
    };
    let results = if query_field(query, "Query") == Some("this-is-just-a-test-for-organizr") {
        json!([])
    } else {
        json!([
            {
                "FirstSeen": "0001-01-01T00:00:00",
                "Tracker": "Tracker one",
                "TrackerId": "tracker",
                "CategoryDesc": "Movies",
                "Title": "A Film 2024 1080p",
                "Guid": "https://tracker.example.com/details/1",
                "Link": link("dl"),
                "Details": "https://tracker.example.com/details/1",
                "PublishDate": "2026-11-01T10:00:00+00:00",
                "Category": [2000],
                "Size": 4_294_967_296_u64,
                "Files": 3,
                "Grabs": 120,
                "Seeders": 45,
                "Peers": 7,
                "MagnetUri": null,
                "BlackholeLink": link("bh"),
            },
            {
                "Tracker": "Tracker two",
                "TrackerId": "other",
                "Title": "A Film 2024 720p",
                "Guid": "magnet:?xt=urn:btih:0000000000000000000000000000000000000000",
                "Link": null,
                "Details": null,
                "PublishDate": "2026-10-20T08:30:00+00:00",
                "Size": 1_073_741_824,
                "Files": null,
                "Grabs": null,
                "Seeders": 3,
                "Peers": 1,
                "MagnetUri": "magnet:?xt=urn:btih:0000000000000000000000000000000000000000",
                "BlackholeLink": null,
            },
        ])
    };
    axum::Json(json!({
        "Results": results,
        "Indexers": [{"ID": "tracker", "Name": "Tracker one", "Status": 2, "Results": 1, "Error": null}],
    }))
    .into_response()
}

/// Jackett answers a black hole link with a result, also when it is an error.
async fn jackett_black_hole(RawQuery(query): RawQuery) -> Response {
    let query = query.as_deref();
    let answer = if query_field(query, "jackett_apikey") != Some(JACKETT_KEY) {
        json!({"result": "error", "error": "Incorrect API key"})
    } else if query_field(query, "path") == Some("made-up-path") {
        json!({"result": "success"})
    } else {
        json!({"result": "error", "error": "Blackhole directory does not exist"})
    };
    axum::Json(answer).into_response()
}

/// A stand-in for Jackett, with two releases for every search but the one of the
/// connection test.
pub async fn jackett() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v2.0/indexers/all/results", get(jackett_results))
            .route("/bh/tracker/", get(jackett_black_hole)),
    )
    .await
}

/// The API key of the Prowlarr stand-in.
pub const PROWLARR_KEY: &str = "made-up-prowlarr-key";

/// The `guid` of the release of the Prowlarr stand-in.
pub const PROWLARR_GUID: &str = "https://tracker.example.com/details/1";

fn has_prowlarr_key(headers: &HeaderMap) -> bool {
    headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        == Some(PROWLARR_KEY)
}

fn prowlarr_release() -> Value {
    json!({
        "guid": PROWLARR_GUID,
        "age": 19,
        "size": 4_294_967_296_u64,
        "indexerId": 5,
        "indexer": "Tracker one",
        "title": "A Film 2024 1080p",
        "sortTitle": "a film 2024 1080p",
        "publishDate": "2026-11-01T10:00:00Z",
        "downloadUrl": format!(
            "http://127.0.0.1:9696/5/download?apikey={PROWLARR_KEY}&link=made-up-link&file=A+Film+2024"
        ),
        "infoUrl": "https://tracker.example.com/details/1",
        "grabs": 120,
        "seeders": 45,
        "leechers": 7,
        "protocol": "torrent",
    })
}

async fn prowlarr_search(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    if !has_prowlarr_key(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Unauthorized"})),
        )
            .into_response();
    }
    if query_field(query.as_deref(), "query") == Some("this-is-just-a-test-for-organizr") {
        return axum::Json(json!([])).into_response();
    }
    axum::Json(json!([prowlarr_release()])).into_response()
}

/// Prowlarr sends a release of its last search to its download client, and answers
/// with the release.
async fn prowlarr_grab(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    if !has_prowlarr_key(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Unauthorized"})),
        )
            .into_response();
    }
    let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    if asked.get("guid") == Some(&json!(PROWLARR_GUID)) && asked.get("indexerId") == Some(&json!(5))
    {
        axum::Json(prowlarr_release()).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            axum::Json(json!({
                "message": "Couldn't find requested release in cache, try searching again",
            })),
        )
            .into_response()
    }
}

/// A stand-in for Prowlarr, with one release for every search but the one of the
/// connection test.
pub async fn prowlarr() -> StandIn {
    StandIn::start(Router::new().route("/api/v1/search", get(prowlarr_search).post(prowlarr_grab)))
        .await
}

async fn adguard_stats() -> Response {
    axum::Json(json!({
        "time_units": "hours",
        "num_dns_queries": 12_000,
        "num_blocked_filtering": 3_000,
        "num_replaced_safebrowsing": 0,
        "num_replaced_safesearch": 0,
        "num_replaced_parental": 0,
        "avg_processing_time": 0.012_34,
        "top_queried_domains": [{"example.com": 400}],
        "top_clients": [{"192.0.2.20": 900}],
        "top_blocked_domains": [{"ads.example.com": 250}],
    }))
    .into_response()
}

async fn adguard_filters() -> Response {
    axum::Json(json!({
        "enabled": true,
        "interval": 24,
        "filters": [
            {"id": 1, "enabled": true, "name": "List one", "rules_count": 100_000,
             "url": "https://lists.example.com/one.txt"},
            {"id": 2, "enabled": true, "name": "List two", "rules_count": 50_000,
             "url": "https://lists.example.com/two.txt"},
        ],
        "whitelist_filters": null,
        "user_rules": ["||ads.example.com^", "@@||good.example.com^"],
    }))
    .into_response()
}

/// A stand-in for AdGuard Home without a login.
pub async fn adguard() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/control/stats", get(adguard_stats))
            .route("/control/filtering/status", get(adguard_filters)),
    )
    .await
}

async fn adguard_refusal() -> Response {
    (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
}

/// A stand-in for AdGuard Home with a login: it refuses a request without one.
pub async fn adguard_with_login() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/control/stats", get(adguard_refusal))
            .route("/control/filtering/status", get(adguard_refusal)),
    )
    .await
}

/// The data API of Netdata.
async fn netdata_data(RawQuery(query): RawQuery) -> Response {
    let query = query.as_deref();
    let chart = query_field(query, "chart").unwrap_or_default();
    let dimension = query_field(query, "dimensions")
        .or_else(|| query_field(query, "dimension"))
        .unwrap_or_default();
    let wrapped = |latest: f64, max: f64| {
        json!({
            "api": 1,
            "id": chart,
            "name": chart,
            "view_update_every": 1,
            "update_every": 1,
            "dimension_names": [dimension],
            "latest_values": [latest],
            "view_latest_values": [latest],
            "dimensions": 1,
            "points": 540,
            "format": "array",
            "result": [latest],
            "min": 0,
            "max": max,
        })
    };
    let answer = match (chart, dimension) {
        ("system.io", "in") => wrapped(1500.0, 6000.0),
        ("system.io", "out") => wrapped(-3000.0, 6000.0),
        ("system.net", "received") => wrapped(8000.0, 100_000.0),
        ("system.net", "sent") => wrapped(-2000.0, 100_000.0),
        ("system.cpu", _) => json!([12.5, 11.0, 14.25]),
        ("system.ram", _) => wrapped(42.5, 60.0),
        ("system.swap", _) => wrapped(1.5, 2.0),
        ("disk_space._", "avail" | "used") => json!({
            "api": 1,
            "id": "disk_space._",
            "format": "json",
            "result": {
                "labels": ["time", dimension],
                "data": [[1_793_000_000_000_u64, if dimension == "avail" { 61.5 } else { 38.5 }]],
            },
            "min": 0,
            "max": 100,
        }),
        _ => {
            return (
                StatusCode::NOT_FOUND,
                format!("Chart is not found: {chart}"),
            )
                .into_response();
        }
    };
    axum::Json(answer).into_response()
}

/// A stand-in for Netdata, with the charts of the system that the homepage asks for.
pub async fn netdata() -> StandIn {
    StandIn::start(Router::new().route("/api/v1/data", get(netdata_data))).await
}

/// The HTML that Monitorr loads into its page: one tile for each service. Two tiles
/// stand in one line.
const MONITORR_TILES: &str = concat!(
    "<div class=\"col-lg-4\"><a class=\"servicetile\" href=\"https://plex.example.com/web\" target=\"_blank\" style=\"display: block\"><div id=\"serviceimg\"><div><img id=\"plex-service-img\" src=\"assets/img/../data/usrimg/plex.png\" class=\"serviceimg\" alt=plex></div></div><div id=\"servicetitle\"><div>Plex</div></div><div class=\"btnonline\">Online</div></a></div></div>",
    "<div class=\"col-lg-4\"><div class=\"servicetile\"><div id=\"serviceimg\"><div><img id=\"router-service-img\" src=\"assets/img/../data/usrimg/router.png\" class=\"serviceimg\" alt=router></div></div><div id=\"servicetitlenolink\" style=\"cursor: default\"><div>Router</div></div><div class=\"btnunknown\">Unresponsive</div></div></div>\n",
    "<div class=\"col-lg-4\"><div class=\"servicetile\"><div id=\"serviceimg\"><div><img id=\"backup &amp; sync-service-img\" src=\"assets/img/../data/usrimg/backup.png\" class=\"serviceimg imgoffline\" alt=backup></div></div><div id=\"servicetitleoffline\" style=\"cursor: default\"><div>Backup &amp; Sync</div></div><div class=\"btnoffline\" style=\"cursor: default\">Offline</div></div></div>\n",
);

async fn monitorr_tiles() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=UTF-8")],
        MONITORR_TILES,
    )
        .into_response()
}

async fn monitorr_picture() -> Response {
    ([(header::CONTENT_TYPE, "image/png")], JPEG).into_response()
}

/// A stand-in for Monitorr with three services. Only the picture of the first one can
/// be loaded.
pub async fn monitorr() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/assets/php/loop.php", get(monitorr_tiles))
            .route("/assets/data/usrimg/plex.png", get(monitorr_picture)),
    )
    .await
}

fn speedtest_result(ping: f64, download: f64, upload: f64) -> Value {
    json!({
        "id": 42,
        "ping": ping,
        "download": download,
        "upload": upload,
        "server_id": 1234,
        "server_name": "A server",
        "server_host": "speedtest.example.com:8080",
        "url": "https://www.speedtest.net/result/c/made-up",
        "scheduled": true,
        "failed": false,
        "created_at": "2026-11-01T10:00:00.000000Z",
        "updated_at": "2026-11-01T10:00:00.000000Z",
    })
}

async fn speedtest_latest() -> Response {
    axum::Json(json!({
        "method": "get latest speedtest",
        "data": speedtest_result(12.345, 250.12, 40.26),
        "average": {"ping": 13.5, "download": 240.0, "upload": 39.0},
        "max": {"ping": 20.0, "download": 260.5, "upload": 41.0},
        "minimum": {"ping": 9.5, "download": 200.25, "upload": 35.0},
    }))
    .into_response()
}

/// A stand-in for Speedtest Tracker.
pub async fn speedtest_tracker() -> StandIn {
    StandIn::start(Router::new().route("/api/speedtest/latest", get(speedtest_latest))).await
}

/// The API key of the OctoPrint stand-in.
pub const OCTOPRINT_KEY: &str = "made-up-octoprint-key";

/// A key in the settings of the OctoPrint stand-in, which must stay on the server.
pub const OCTOPRINT_INNER_KEY: &str = "made-up-key-inside-the-settings";

fn for_octoprint_key(headers: &HeaderMap, body: Value) -> Response {
    let key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    if key == Some(OCTOPRINT_KEY) {
        axum::Json(body).into_response()
    } else {
        (StatusCode::FORBIDDEN, "Invalid API key").into_response()
    }
}

async fn octoprint_job(headers: HeaderMap) -> Response {
    for_octoprint_key(
        &headers,
        json!({
            "job": {
                "file": {"name": "bracket.gcode", "display": "Bracket.gcode", "origin": "local", "size": 1_234_567},
                "estimatedPrintTime": 5400.0,
                "filament": {"tool0": {"length": 3200.5, "volume": 7.7}},
                "user": "admin",
            },
            "progress": {"completion": 41.6, "filepos": 513_580, "printTime": 2200, "printTimeLeft": 3200},
            "state": "Printing",
        }),
    )
}

async fn octoprint_settings(headers: HeaderMap) -> Response {
    for_octoprint_key(
        &headers,
        json!({
            "api": {"allowCrossOrigin": false, "key": OCTOPRINT_INNER_KEY},
            "appearance": {"name": "Printer"},
            "webcam": {
                "webcamEnabled": true,
                "streamUrl": "/webcam/?action=stream",
                "snapshotUrl": "http://127.0.0.1:8080/?action=snapshot",
                "flipH": false,
            },
        }),
    )
}

/// A stand-in for OctoPrint, with a printer that prints.
pub async fn octoprint() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/job", get(octoprint_job))
            .route("/api/settings", get(octoprint_settings)),
    )
    .await
}

/// The account of the UniFi stand-ins.
pub const UNIFI_LOGIN: (&str, &str) = ("organizr", "the password of UniFi");

/// The token that the stand-in for a console with UniFi OS sends with its start page.
pub const UNIFI_TOKEN: &str = "made-up-csrf-token";

const UNIFI_CLASSIC_COOKIES: &str = "unifises=made-up-session; csrf_token=made-up-classic-token";
const UNIFI_OS_COOKIE: &str = "TOKEN=made-up-token-of-the-console";

fn unifi_health_body() -> Value {
    json!({
        "meta": {"rc": "ok"},
        "data": [
            {"subsystem": "wlan", "status": "ok", "num_user": 14, "num_guest": 2, "num_ap": 3,
             "tx_bytes-r": 120_000, "rx_bytes-r": 45_000},
            {"subsystem": "wan", "status": "ok", "wan_ip": "203.0.113.7", "num_gw": 1,
             "tx_bytes-r": 300_000, "rx_bytes-r": 900_000},
            {"subsystem": "www", "status": "ok", "drops": 2, "latency": 11, "uptime": 86_400,
             "tx_bytes-r": 300_000, "rx_bytes-r": 900_000},
            {"subsystem": "lan", "status": "ok", "num_user": 9, "num_sw": 2,
             "tx_bytes-r": 80_000, "rx_bytes-r": 60_000},
            {"subsystem": "vpn", "status": "unknown"},
        ],
    })
}

fn unifi_refusal(status: StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(json!({"meta": {"rc": "error", "msg": message}, "data": []})),
    )
        .into_response()
}

fn has_cookies(headers: &HeaderMap, wanted: &str) -> bool {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        == Some(wanted)
}

/// The classic controller sends its start page from `/manage`.
async fn unifi_classic_root() -> Response {
    (StatusCode::FOUND, [(header::LOCATION, "/manage")]).into_response()
}

async fn unifi_classic_page() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html")],
        "<html><title>UniFi Network</title></html>",
    )
        .into_response()
}

async fn unifi_classic_login(body: axum::body::Bytes) -> Response {
    let asked: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    if asked.get("username") == Some(&json!(UNIFI_LOGIN.0))
        && asked.get("password") == Some(&json!(UNIFI_LOGIN.1))
    {
        let mut response = axum::Json(json!({"meta": {"rc": "ok"}, "data": []})).into_response();
        for cookie in UNIFI_CLASSIC_COOKIES.split("; ") {
            if let Ok(value) = format!("{cookie}; Path=/; Secure; HttpOnly").parse() {
                response.headers_mut().append(header::SET_COOKIE, value);
            }
        }
        response
    } else {
        unifi_refusal(StatusCode::BAD_REQUEST, "api.err.Invalid")
    }
}

async fn unifi_classic_health(
    headers: HeaderMap,
    axum::extract::Path(site): axum::extract::Path<String>,
) -> Response {
    if !has_cookies(&headers, UNIFI_CLASSIC_COOKIES) {
        return unifi_refusal(StatusCode::UNAUTHORIZED, "api.err.LoginRequired");
    }
    if site != "default" {
        return unifi_refusal(StatusCode::UNAUTHORIZED, "api.err.NoSiteContext");
    }
    axum::Json(unifi_health_body()).into_response()
}

async fn unifi_classic_sites(headers: HeaderMap) -> Response {
    if !has_cookies(&headers, UNIFI_CLASSIC_COOKIES) {
        return unifi_refusal(StatusCode::UNAUTHORIZED, "api.err.LoginRequired");
    }
    axum::Json(json!({
        "meta": {"rc": "ok"},
        "data": [
            {"_id": "made-up-id-one", "name": "default", "desc": "Default", "role": "admin"},
            {"_id": "made-up-id-two", "name": "x7k2m9qa", "desc": "Second house", "role": "admin"},
        ],
    }))
    .into_response()
}

/// A stand-in for the classic UniFi Network controller, with the sites `default` and
/// `x7k2m9qa`. Only the first one has data.
pub async fn unifi_classic() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/", get(unifi_classic_root))
            .route("/manage", get(unifi_classic_page))
            .route("/api/login", axum::routing::post(unifi_classic_login))
            .route("/api/s/{site}/stat/health", get(unifi_classic_health))
            .route("/api/self/sites", get(unifi_classic_sites)),
    )
    .await
}

async fn unifi_os_root() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/html"),
            (header::HeaderName::from_static("x-csrf-token"), UNIFI_TOKEN),
        ],
        "<html><title>UniFi OS</title></html>",
    )
        .into_response()
}

fn has_unifi_token(headers: &HeaderMap) -> bool {
    headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        == Some(UNIFI_TOKEN)
}

/// The console takes the login as the fields of a form, as the PHP application sends
/// it, with `+` for a space.
async fn unifi_os_login(headers: HeaderMap, body: axum::body::Bytes) -> Response {
    let body = String::from_utf8_lossy(&body).into_owned();
    let field = |name: &str| {
        query_field(Some(&body), name)
            .unwrap_or_default()
            .replace('+', " ")
    };
    if has_unifi_token(&headers)
        && field("username") == UNIFI_LOGIN.0
        && field("password") == UNIFI_LOGIN.1
    {
        (
            [(
                header::SET_COOKIE,
                format!("{UNIFI_OS_COOKIE}; path=/; samesite=none; secure; httponly"),
            )],
            axum::Json(json!({"unique_id": "made-up-id", "username": UNIFI_LOGIN.0})),
        )
            .into_response()
    } else {
        (
            StatusCode::FORBIDDEN,
            axum::Json(json!({"code": "AUTHENTICATION_FAILED_INVALID_CREDENTIALS"})),
        )
            .into_response()
    }
}

async fn unifi_os_health(headers: HeaderMap) -> Response {
    if has_cookies(&headers, UNIFI_OS_COOKIE) && has_unifi_token(&headers) {
        axum::Json(unifi_health_body()).into_response()
    } else {
        unifi_refusal(StatusCode::UNAUTHORIZED, "api.err.LoginRequired")
    }
}

/// A stand-in for a console with UniFi OS.
pub async fn unifi_os() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/", get(unifi_os_root))
            .route("/api/auth/login", axum::routing::post(unifi_os_login))
            .route(
                "/proxy/network/api/s/default/stat/health",
                get(unifi_os_health),
            ),
    )
    .await
}

/// The client id and the client secret of the API application at the stand-in for
/// Trakt.
pub const TRAKT_CLIENT: (&str, &str) = ("made-up-trakt-client-id", "made-up-trakt-client-secret");

/// The code that the stand-in for Trakt takes for tokens.
pub const TRAKT_CODE: &str = "made-up-trakt-code";

/// The access token and the refresh token that the stand-in for Trakt gives for
/// [`TRAKT_CODE`].
pub const TRAKT_TOKENS: (&str, &str) =
    ("made-up-trakt-access-token", "made-up-trakt-refresh-token");

/// The refresh token that the stand-in for Trakt gives for a refresh token.
pub const TRAKT_NEXT_REFRESH: &str = "made-up-trakt-refresh-token-two";

fn has_trakt_token(headers: &HeaderMap) -> bool {
    let sent = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let has_key = headers
        .get("trakt-api-key")
        .and_then(|value| value.to_str().ok())
        == Some(TRAKT_CLIENT.0);
    has_key && sent == Some(format!("Bearer {}", TRAKT_TOKENS.0).as_str())
}

async fn trakt_shows(headers: HeaderMap) -> Response {
    if !has_trakt_token(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    axum::Json(json!([
        {
            "first_aired": "2099-01-05T02:00:00.000Z",
            "episode": {"season": 3, "number": 1, "title": "The Return", "overview": "They are back.", "runtime": 45, "ids": {"tmdb": 9001}},
            "show": {"title": "The Show", "year": 2019, "rating": 8.1, "genres": ["drama"], "ids": {"tmdb": 4711}},
        },
        {
            "first_aired": "2001-03-04T20:00:00.000Z",
            "episode": {"season": 1, "number": 12, "title": "The End", "ids": {"tmdb": 9002}},
            "show": {"title": "The Old Show", "ids": {"tmdb": 4712}},
        },
    ]))
    .into_response()
}

async fn trakt_movies(headers: HeaderMap) -> Response {
    if !has_trakt_token(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    axum::Json(json!([
        {
            "released": "2099-01-07",
            "movie": {
                "title": "The Film", "year": 2099, "released": "2099-01-07", "tagline": "One more time",
                "status": "released", "overview": "A film.", "runtime": 110, "rating": 7.5,
                "genres": ["action"], "ids": {"tmdb": 815},
            },
        },
    ]))
    .into_response()
}

async fn trakt_token(axum::Json(body): axum::Json<Value>) -> Response {
    let field = |name: &str| body.get(name).and_then(Value::as_str).unwrap_or_default();
    let is_application = field("client_id") == TRAKT_CLIENT.0
        && field("client_secret") == TRAKT_CLIENT.1
        && field("redirect_uri").ends_with("/api/v2/oauth/trakt");
    let refresh = match (field("grant_type"), is_application) {
        ("authorization_code", true) if field("code") == TRAKT_CODE => TRAKT_TOKENS.1,
        ("refresh_token", true) if field("refresh_token") == TRAKT_TOKENS.1 => TRAKT_NEXT_REFRESH,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                axum::Json(json!({
                    "error": "invalid_grant",
                    "error_description": "The provided authorization grant is invalid, expired or revoked.",
                })),
            )
                .into_response();
        }
    };
    axum::Json(json!({
        "access_token": TRAKT_TOKENS.0,
        "token_type": "bearer",
        "expires_in": 86400,
        "refresh_token": refresh,
        "scope": "public",
        // The first day of 2100, so the token is not over while a test runs.
        "created_at": 4_102_444_800_i64,
    }))
    .into_response()
}

/// A stand-in for the API of Trakt: the calendar of one account with two episodes
/// and one film, and the route that gives tokens.
pub async fn trakt() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/calendars/my/shows/{start}/{days}", get(trakt_shows))
            .route("/calendars/my/movies/{start}/{days}", get(trakt_movies))
            .route("/oauth/token", axum::routing::post(trakt_token)),
    )
    .await
}

/// The API key of the stand-in for JellyStat.
pub const JELLYSTAT_KEY: &str = "made-up-jellystat-key";

fn has_jellystat_key(headers: &HeaderMap) -> bool {
    headers
        .get("x-api-token")
        .and_then(|value| value.to_str().ok())
        == Some(JELLYSTAT_KEY)
}

fn jellystat_refusal() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(json!({"error": "Invalid API Key"})),
    )
        .into_response()
}

async fn jellystat_libraries(headers: HeaderMap) -> Response {
    if !has_jellystat_key(&headers) {
        return jellystat_refusal();
    }
    axum::Json(json!([
        {"Id": "lib-1", "Name": "Films", "ServerId": "made-up-server-id", "CollectionType": "movies", "item_count": 120, "season_count": 0, "episode_count": 0, "total_play_time": 36_610_000_000_i64},
        {"Id": "lib-2", "Name": "Series", "ServerId": "made-up-server-id", "CollectionType": "tvshows", "item_count": 40, "season_count": 90, "episode_count": 800, "total_play_time": 0},
    ]))
    .into_response()
}

async fn jellystat_history(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    if !has_jellystat_key(&headers) {
        return jellystat_refusal();
    }
    if query_field(query.as_deref(), "page") != Some("1") {
        return axum::Json(json!({"results": []})).into_response();
    }
    let video = json!([{"Type": "Video"}, {"Type": "Audio"}]);
    axum::Json(json!({
        "current_page": 1,
        "pages": 1,
        "results": [
            {"Id": "h1", "UserName": "robin", "NowPlayingItemId": "815", "NowPlayingItemName": "The Film (2019)", "PlaybackDuration": 5400, "ActivityDateInserted": "2026-11-18T20:00:00.000Z", "ServerId": "made-up-server-id", "MediaStreams": video},
            {"Id": "h2", "UserName": "robin", "NowPlayingItemId": "301", "NowPlayingItemName": "The One", "SeriesName": "The Show", "EpisodeId": "301", "PlaybackDuration": 1800, "ActivityDateInserted": "2026-11-19T20:00:00.000Z", "ServerId": "made-up-server-id", "MediaStreams": video},
            {"Id": "h3", "UserName": "pat", "NowPlayingItemId": "302", "NowPlayingItemName": "The Two", "SeriesName": "The Show", "EpisodeId": "302", "PlaybackDuration": 1700, "ActivityDateInserted": "2026-11-17T20:00:00.000Z", "ServerId": "made-up-server-id", "MediaStreams": video},
            {"Id": "h4", "UserName": "pat", "NowPlayingItemId": "77", "NowPlayingItemName": "A Song", "PlaybackDuration": 200, "ActivityDateInserted": "2026-11-16T20:00:00.000Z", "ServerId": "made-up-server-id", "MediaStreams": [{"Type": "Audio"}]},
        ],
    }))
    .into_response()
}

async fn jellystat_item(headers: HeaderMap, RawQuery(query): RawQuery) -> Response {
    if !has_jellystat_key(&headers) {
        return jellystat_refusal();
    }
    if query_field(query.as_deref(), "id") != Some("815") {
        return (
            StatusCode::NOT_FOUND,
            axum::Json(json!({"error": "not found"})),
        )
            .into_response();
    }
    axum::Json(json!({
        "Id": "815", "ServerId": "made-up-server-id", "Type": "Movie", "Name": "The Film",
        "ProductionYear": 2019, "Overview": "A film, as JellyStat knows it.",
        "ImageTags": {"Primary": "made-up-tag"},
    }))
    .into_response()
}

/// A stand-in for JellyStat: two libraries, a history of four rows, what it knows
/// about the item 815, and a page at its address. The key comes in the header
/// `x-api-token`.
pub async fn jellystat() -> StandIn {
    StandIn::start(
        Router::new()
            .route(
                "/",
                get(|| async { axum::response::Html("<html><title>Jellystat</title></html>") }),
            )
            .route("/api/getLibraries", get(jellystat_libraries))
            .route("/api/getHistory", get(jellystat_history))
            .route("/api/getItem", get(jellystat_item)),
    )
    .await
}

fn live_tv_api(headers: &HeaderMap, uri: &axum::http::Uri) -> Response {
    let sent = headers
        .get("x-emby-token")
        .and_then(|value| value.to_str().ok());
    if sent != Some(MEDIA_SERVER_KEY) {
        return (
            StatusCode::UNAUTHORIZED,
            "Access token is invalid or expired.",
        )
            .into_response();
    }
    let body = match uri.path() {
        "/emby/System/Info" => {
            json!({"ServerName": "Made-up Emby", "Version": "4.8.0.0", "Id": "made-up-server-id"})
        }
        "/emby/LiveTv/Info" => {
            json!({"Services": [{"Name": "Emby", "Status": "Ok"}], "IsEnabled": true})
        }
        "/emby/Users" => json!([
            {"Id": "1", "Name": "Owner", "Policy": {"IsAdministrator": true}},
            {"Id": "2", "Name": "Zed", "Policy": {"IsAdministrator": false}},
        ]),
        "/emby/LiveTv/Timers" => json!({"TotalRecordCount": 2, "Items": [
            {
                "Id": "t2", "Name": "Late Show", "ChannelId": "12", "UserId": "2",
                "StartDate": "2099-01-05T22:00:00.0000000Z", "EndDate": "2099-01-05T23:30:00.0000000Z",
                "ProgramInfo": {"Name": "Late Show", "EpisodeTitle": "Guests", "SeriesName": "The Late Show"},
            },
            {
                "Id": "t1", "Name": "News", "SeriesName": "The News", "ChannelName": "One",
                "StartDate": "2001-03-04T20:00:00.0000000Z", "EndDate": "2001-03-04T20:30:00.0000000Z",
            },
        ]}),
        "/emby/LiveTv/SeriesTimers" => {
            json!({"TotalRecordCount": 1, "Items": [{"Id": "st1", "Name": "The News"}]})
        }
        "/emby/LiveTv/Recordings" => json!({"TotalRecordCount": 31, "Items": [
            {"Id": "r1", "Name": "The Film", "ChannelName": "Two", "DateCreated": "2026-11-18T20:15:00.0000000Z", "RunTimeTicks": 54_000_000_000_i64},
            {"Id": "r2", "Name": "News", "SeriesName": "The News", "DateCreated": "2026-11-17T06:00:00.0000000Z"},
        ]}),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    axum::Json(body).into_response()
}

/// A stand-in for an Emby server with live television: its name, two timers, one
/// series timer, two recordings and two users. The key comes in the header
/// `X-Emby-Token`.
pub async fn emby_live_tv() -> StandIn {
    StandIn::start(
        Router::new().fallback(|headers: HeaderMap, uri: axum::http::Uri| {
            std::future::ready(live_tv_api(&headers, &uri))
        }),
    )
    .await
}

/// A stand-in for PromPage, with two services.
pub async fn prompage() -> StandIn {
    StandIn::start(Router::new().route(
        "/api/services",
        get(|| async {
            axum::Json(json!({"services": [
                {"name": "Blog", "group": "Web", "status": true, "uptime": 99.954},
                {"name": "Mail", "group": "Web", "status": false, "uptime": 87.5},
            ]}))
        }),
    ))
    .await
}

/// The secret key of the account at the stand-in for Stripe.
pub const STRIPE_SECRET: &str = "sk_test_made-up-stripe-secret";

/// The page for a payment that the stand-in for Stripe names.
pub const STRIPE_PAGE: &str = "https://checkout.stripe.example/c/pay/cs_test_made-up";

fn stripe_refusal(headers: &HeaderMap) -> Option<Response> {
    let sent = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    (sent != Some(format!("Bearer {STRIPE_SECRET}").as_str())).then(|| {
        (
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": {"type": "invalid_request_error", "message": "Invalid API Key provided: sk_test_****"}})),
        )
            .into_response()
    })
}

async fn stripe_charges(headers: HeaderMap) -> Response {
    if let Some(refusal) = stripe_refusal(&headers) {
        return refusal;
    }
    let charge = |id: &str, cents: u32, email: &str| {
        json!({
            "id": id, "object": "charge", "amount": cents, "amount_captured": cents,
            // 20 November 2026, 10:00 UTC.
            "created": 1_795_168_800, "currency": "usd",
            "billing_details": {"email": email, "name": "A Name"},
        })
    };
    axum::Json(json!({"object": "list", "has_more": false, "data": [
        charge("ch_1", 1250, "Admin@Example.com"),
        charge("ch_2", 500, "Zoe@Example.com"),
        charge("ch_3", 0, "zoe@example.com"),
    ]}))
    .into_response()
}

async fn stripe_session(headers: HeaderMap) -> Response {
    if let Some(refusal) = stripe_refusal(&headers) {
        return refusal;
    }
    axum::Json(json!({"id": "cs_test_made-up", "object": "checkout.session", "url": STRIPE_PAGE}))
        .into_response()
}

/// A stand-in for the API of Stripe: three charges, and the start of a payment.
pub async fn stripe() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/v1/charges", get(stripe_charges))
            .route("/v1/checkout/sessions", axum::routing::post(stripe_session)),
    )
    .await
}

/// The API key of the stand-in for YouTube.
pub const YOUTUBE_KEY: &str = "made-up-youtube-key";

async fn youtube_search(headers: HeaderMap) -> Response {
    let sent = headers
        .get("x-goog-api-key")
        .and_then(|value| value.to_str().ok());
    if sent != Some(YOUTUBE_KEY) {
        return (
            StatusCode::FORBIDDEN,
            axum::Json(json!({"error": {"code": 403, "message": "The request is missing a valid API key."}})),
        )
            .into_response();
    }
    axum::Json(json!({
        "kind": "youtube#searchListResponse",
        "items": [{
            "kind": "youtube#searchResult",
            "id": {"kind": "youtube#video", "videoId": "made-up-video"},
            "snippet": {"title": "The Film - Official Trailer"},
        }],
    }))
    .into_response()
}

/// A stand-in for the API of YouTube, which finds one video.
pub async fn youtube() -> StandIn {
    StandIn::start(Router::new().route("/youtube/v3/search", get(youtube_search))).await
}

/// A stand-in for a web server with pages: a page of HTML, a file of JSON, a page
/// that forbids frames, a page that sends the browser on, and a webhook.
pub async fn pages() -> StandIn {
    StandIn::start(
        Router::new()
            .route(
                "/page",
                get(|| async { axum::response::Html("<p>Fish &amp; Chips &copy; 2026</p>") }),
            )
            .route(
                "/data.json",
                get(|| async { axum::Json(json!({"answer": 42})) }),
            )
            .route(
                "/no-frames",
                get(|| async { ([("x-frame-options", "DENY")], "no frames") }),
            )
            .route(
                "/same-origin",
                get(|| async { ([("x-frame-options", "SAMEORIGIN")], "same origin") }),
            )
            .route(
                "/moved",
                get(|| async { (StatusCode::FOUND, [(header::LOCATION, "/page")]) }),
            )
            .route("/hook", axum::routing::post(|| async { "ok" }))
            .route(
                "/closed-hook",
                axum::routing::post(|| async { (StatusCode::NOT_FOUND, "no such hook") }),
            ),
    )
    .await
}

/// What the stand-in of an application behind the API SOCKS answers on every path but
/// `/teapot` and `/moved`.
pub const SOCKS_ANSWER: &str = "{\"records\":[{\"title\":\"The Show\"}]}";

/// A stand-in for an application behind the API SOCKS, such as Sonarr. It answers every
/// path and method with [`SOCKS_ANSWER`] as JSON, `/teapot` with the status 418 and
/// HTML, and `/moved` with a redirect. Its log shows what Organizr passed on.
pub async fn socks_app() -> StandIn {
    StandIn::start(
        Router::new()
            .route(
                "/teapot",
                axum::routing::any(|| async {
                    (
                        StatusCode::IM_A_TEAPOT,
                        [(header::CONTENT_TYPE, "text/html")],
                        "<p>short and stout</p>",
                    )
                }),
            )
            .route(
                "/moved",
                axum::routing::any(|| async {
                    (
                        StatusCode::FOUND,
                        [(header::LOCATION, "http://other.example/")],
                        "",
                    )
                }),
            )
            .fallback(|| async { ([(header::CONTENT_TYPE, "application/json")], SOCKS_ANSWER) }),
    )
    .await
}
