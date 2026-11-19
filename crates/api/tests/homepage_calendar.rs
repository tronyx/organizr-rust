//! The calendar of the homepage and the download queues, against stand-ins for
//! Sonarr, Radarr and Lidarr.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, ARR_KEY, JPEG};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_in, installed_app_with, seed};
use serde_json::{Value, json};

const FAN_ART: &str = "/MediaCover/1/fanart.jpg";

/// The settings of an install that shows the three applications.
fn settings(sonarr: &StandIn, radarr: &StandIn, lidarr: &StandIn) -> Vec<(&'static str, Value)> {
    vec![
        ("homepageSonarrEnabled", json!(true)),
        ("sonarrURL", json!(sonarr.url())),
        ("sonarrToken", json!(ARR_KEY)),
        ("homepageRadarrEnabled", json!(true)),
        ("homepageRadarrAuth", json!("4")),
        ("radarrURL", json!(radarr.url())),
        ("radarrToken", json!(ARR_KEY)),
        ("homepageLidarrEnabled", json!(true)),
        ("lidarrURL", json!(lidarr.url())),
        ("lidarrToken", json!(ARR_KEY)),
    ]
}

fn ids(events: &Value) -> Vec<String> {
    events
        .as_array()
        .expect("a list of events")
        .iter()
        .map(|event| event["id"].as_str().unwrap_or("?").to_owned())
        .collect()
}

/// The kinds of the items that the homepage lists for a caller.
async fn kinds(app: &Router, who: As<'_>) -> Vec<String> {
    call(app, who, "GET", "/homepage/items", None)
        .await
        .data()
        .as_array()
        .expect("a list of items")
        .iter()
        .map(|item| item["kind"].as_str().unwrap_or("?").to_owned())
        .collect()
}

#[tokio::test]
async fn calendar_holds_the_events_of_each_application_that_the_caller_sees() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (sonarr, radarr, lidarr) = (
            homepage_services::sonarr().await,
            homepage_services::radarr().await,
            homepage_services::lidarr().await,
        );
        let app = installed_app_with(database, &settings(&sonarr, &radarr, &lidarr));
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let all = call(&app, As::Admin, "GET", "/homepage/calendar", None).await;

        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(all.data()["ical"], json!(false));
        // Sonarr, then Lidarr, then Radarr, as in the PHP application.
        assert_eq!(
            ids(&all.data()["events"]),
            ["Sonarr-0-1", "Lidarr-0-1", "Radarr-0-1"]
        );
        let film = &all.data()["events"][2];
        assert_eq!(film["start"], "2020-01-07");
        assert_eq!(film["details"]["image"], "https://art.example.net/film.jpg");
        assert_eq!(
            film["details"]["href"],
            format!("{}/movie/815", radarr.url())
        );

        // The query names the days around today, and Sonarr is asked for its series.
        let asked = &sonarr.seen()[0];
        assert_eq!(asked.header("x-api-key"), Some(ARR_KEY));
        assert!(
            asked.path.starts_with("/api/v3/calendar?start=20"),
            "{}",
            asked.path
        );
        assert!(asked.path.contains("&end=20"), "{}", asked.path);
        assert!(
            asked.path.ends_with("&includeSeries=true"),
            "{}",
            asked.path
        );
        let asked = &lidarr.seen()[0];
        assert!(
            asked.path.starts_with("/api/v1/calendar?"),
            "{}",
            asked.path
        );
        assert!(
            asked.path.ends_with("&includeArtist=true"),
            "{}",
            asked.path
        );

        // A user has the level of Radarr only. A guest has none, and gets a success.
        let some = call(&app, As::User(&user), "GET", "/homepage/calendar", None).await;
        assert_eq!(ids(&some.data()["events"]), ["Radarr-0-1"]);
        let none = call(&app, As::Guest, "GET", "/homepage/calendar", None).await;
        assert_eq!(none.status, StatusCode::OK);
        assert_eq!(none.data(), json!({"ical": false, "events": []}));

        // The list of the homepage has the calendar for the first two only.
        assert_eq!(kinds(&app, As::Admin).await, ["calendar"]);
        assert_eq!(kinds(&app, As::User(&user)).await, ["calendar"]);
        assert_eq!(kinds(&app, As::Guest).await, Vec::<String>::new());
    })
    .await;
}

#[tokio::test]
async fn calendar_of_one_application_takes_dates_and_is_refused_as_an_item() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (sonarr, radarr, lidarr) = (
            homepage_services::sonarr().await,
            homepage_services::radarr().await,
            homepage_services::lidarr().await,
        );
        let mut changed = settings(&sonarr, &radarr, &lidarr);
        changed.push(("sonarrUnmonitored", json!(true)));
        // Two installs of Radarr: the second one has a wrong key and is left out.
        changed.push((
            "radarrURL",
            json!(format!("{},{}", radarr.url(), radarr.url())),
        ));
        changed.push(("radarrToken", json!(format!("{ARR_KEY},a-wrong-key"))));
        let app = installed_app_with(database, &changed);

        let path = "/homepage/sonarr/calendar?start=2020-01-01&end=2020-01-31";
        let episodes = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(episodes.status, StatusCode::OK, "{}", episodes.body);
        assert_eq!(ids(&episodes.data()), ["Sonarr-0-1"]);
        assert_eq!(
            sonarr.seen()[0].path,
            "/api/v3/calendar?start=2020-01-01&end=2020-01-31&unmonitored=true&includeSeries=true"
        );

        let films = call(&app, As::Admin, "GET", "/homepage/radarr/calendar", None).await;
        assert_eq!(ids(&films.data()), ["Radarr-0-1"]);
        assert_eq!(radarr.seen().len(), 2);

        let albums = call(&app, As::Admin, "GET", "/homepage/lidarr/calendar", None).await;
        assert_eq!(ids(&albums.data()), ["Lidarr-0-1"]);

        let refused = call(&app, As::Guest, "GET", "/homepage/sonarr/calendar", None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let off = installed_app_with(database, &[]);
        let refused = call(&off, As::Admin, "GET", "/homepage/lidarr/calendar", None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("homepageLidarrEnabled module is not enabled")
        );
    })
    .await;
}

#[tokio::test]
async fn queue_needs_its_own_switch_and_holds_the_downloads() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (sonarr, radarr, lidarr) = (
            homepage_services::sonarr().await,
            homepage_services::radarr().await,
            homepage_services::lidarr().await,
        );
        let mut changed = settings(&sonarr, &radarr, &lidarr);
        let app = installed_app_with(database, &changed);

        let off = call(&app, As::Admin, "GET", "/homepage/sonarr/queue", None).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageSonarrQueueEnabled module is not enabled")
        );

        changed.push(("homepageSonarrQueueEnabled", json!(true)));
        changed.push(("homepageRadarrQueueEnabled", json!(true)));
        let app = installed_app_with(database, &changed);
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let queue = call(&app, As::Admin, "GET", "/homepage/sonarr/queue", None).await;
        assert_eq!(queue.status, StatusCode::OK, "{}", queue.body);
        let content = &queue.data()["content"];
        assert_eq!(content["historyItems"], json!(false));
        assert_eq!(content["queueItems"][0]["title"], "The.Show.S02E03.1080p");
        assert_eq!(content["queueItems"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            sonarr.seen()[0].path,
            "/api/v3/queue?includeUnknownSeriesItems=false&pageSize=1000&includeSeries=true&includeEpisode=true"
        );

        let empty = call(&app, As::Admin, "GET", "/homepage/radarr/queue", None).await;
        assert_eq!(
            empty.data(),
            json!({"content": {"queueItems": [], "historyItems": false}})
        );
        assert_eq!(
            radarr.seen()[0].path,
            "/api/v3/queue?includeUnknownSeriesItems=false&pageSize=1000"
        );

        // The user has the level of Radarr, and not the level of its queue.
        let refused = call(&app, As::User(&user), "GET", "/homepage/radarr/queue", None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        assert_eq!(
            kinds(&app, As::Admin).await,
            ["calendar", "sonarrqueue", "radarrqueue"]
        );
    })
    .await;
}

async fn picture(app: &Router, path: &str) -> Answer {
    send(app, request("GET", path, &[], "")).await
}

/// How often the stand-in was asked for the fan art.
fn fetches(sonarr: &StandIn) -> usize {
    sonarr
        .seen()
        .iter()
        .filter(|seen| seen.path == FAN_ART)
        .count()
}

async fn fan_art_goes_into_the_cache(database: &Database) {
    seed::installed(database).await;
    let sonarr = homepage_services::sonarr().await;
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let app = installed_app_in(database, folder.path());
    let saved = call(
        &app,
        As::Admin,
        "PUT",
        "/config",
        Some(json!({
            "homepageSonarrEnabled": true,
            "sonarrURL": sonarr.url(),
            "sonarrToken": ARR_KEY,
        })),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let address = "/data/cache/tv-4711.jpg";
    assert_eq!(picture(&app, address).await.status, StatusCode::NOT_FOUND);

    let episodes = call(&app, As::Admin, "GET", "/homepage/sonarr/calendar", None).await;
    assert_eq!(episodes.status, StatusCode::OK, "{}", episodes.body);
    assert_eq!(
        episodes.data()[0]["details"]["image"],
        "data/cache/tv-4711.jpg"
    );

    // The server fetches the picture in the background, with the key of Sonarr.
    let mut answer = picture(&app, address).await;
    for _ in 0..100 {
        if answer.status == StatusCode::OK {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        answer = picture(&app, address).await;
    }
    assert_eq!(answer.status, StatusCode::OK);
    let header = |name: &str| {
        answer
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
    };
    assert_eq!(header("content-type"), Some("image/jpeg"));
    assert_eq!(header("x-content-type-options"), Some("nosniff"));
    let file = folder.path().join("cache").join("tv-4711.jpg");
    assert_eq!(std::fs::read(file).expect("the file is there"), JPEG);
    let asked = sonarr
        .seen()
        .into_iter()
        .find(|seen| seen.path == FAN_ART)
        .expect("the picture was asked for");
    assert_eq!(asked.header("x-api-key"), Some(ARR_KEY));

    // A second calendar does not fetch the picture again.
    call(&app, As::Admin, "GET", "/homepage/sonarr/calendar", None).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(fetches(&sonarr), 1);

    // A name that could leave the folder is no picture.
    for name in ["..%2Fsecret.key", "tv-4711.png", "tv-4711"] {
        let refused = picture(&app, &format!("/data/cache/{name}")).await;
        assert_eq!(refused.status, StatusCode::NOT_FOUND, "{name}");
    }
}

#[tokio::test]
async fn fan_art_that_only_sonarr_has_is_fetched_into_the_cache() {
    on_each_engine(fan_art_goes_into_the_cache).await;
}
