//! The helpers of the homepage that ask the wide web: the search for coordinates,
//! the search for a trailer, and the fetch of a page.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, YOUTUBE_KEY};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app, installed_settings, seed, state_with};
use serde_json::{Value, json};

const COORDINATES: &str = "/homepage/weather/coordinates";
const SCRAPE: &str = "/homepage/scrape";

#[tokio::test]
async fn search_for_coordinates_finds_nothing_and_is_for_an_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let query = Some(json!({"query": "Berlin"}));

        let guest = call(&app, As::Guest, "POST", COORDINATES, query.clone()).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(&app, As::User(&user), "POST", COORDINATES, query.clone()).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);

        for none in [json!({}), json!({"query": ""}), json!({"query": null})] {
            let answer = call(&app, As::Admin, "POST", COORDINATES, Some(none)).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(answer.message(), json!("Query was not supplied"));
        }
        // The answer of the PHP application for a search that Mapbox refuses.
        let answer = call(&app, As::Admin, "POST", COORDINATES, query).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.json()["response"]["result"], "success");
        assert_eq!(answer.data(), Value::Null);
    })
    .await;
}

fn with_youtube(database: &Database, api: &str, key: &str) -> Router {
    let settings = installed_settings(&[("youtubeAPI", json!(key))]);
    let state = state_with(database, settings).with_youtube(api);
    router(state).expect("the route table is valid")
}

#[tokio::test]
async fn trailer_is_searched_with_the_key_of_the_admin_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let youtube = homepage_services::youtube().await;
        let path = "/homepage/youtube/The%20Film";

        // Without a key of the Admin, YouTube is not asked.
        let none = with_youtube(database, &youtube.url(), "");
        let answer = call(&none, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Bad response from YouTube"));
        assert_eq!(youtube.seen().len(), 0);

        // The route is for everybody, as in the PHP application.
        let app = with_youtube(database, &youtube.url(), YOUTUBE_KEY);
        let answer = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["items"][0]["id"]["videoId"], "made-up-video");
        let seen = youtube.seen();
        assert_eq!(
            seen[0].path,
            "/youtube/v3/search?part=snippet&q=The+Film+official+trailer&maxResults=1&type=video&videoDuration=short"
        );
        assert_eq!(seen[0].header("x-goog-api-key"), Some(YOUTUBE_KEY));

        let answer = call(&app, As::Guest, "GET", "/homepage/youtube/0", None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("No query supplied"));

        // YouTube refuses the key, and YouTube is not there.
        let wrong = with_youtube(database, &youtube.url(), "a-wrong-key");
        let answer = call(&wrong, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Bad response from YouTube"));
        let away = with_youtube(database, "http://127.0.0.1:1", YOUTUBE_KEY);
        let answer = call(&away, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Bad response from YouTube"));
        assert!(!answer.body.contains(YOUTUBE_KEY));
    })
    .await;
}

#[tokio::test]
async fn page_of_this_computer_or_of_a_private_network_is_not_fetched() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pages = homepage_services::pages().await;
        let app = installed_app(database);
        let fetch =
            async |who: As<'_>, body: Value| call(&app, who, "POST", SCRAPE, Some(body)).await;
        let local = json!({"url": format!("{}/page", pages.url())});

        // The PHP application lets everybody fetch. Here an Admin or a Co-Admin does.
        assert_eq!(
            fetch(As::Guest, local.clone()).await.status,
            StatusCode::UNAUTHORIZED
        );
        let user = seed::logged_in(database, &app, "uma", 4).await;
        assert_eq!(
            fetch(As::User(&user), local.clone()).await.status,
            StatusCode::UNAUTHORIZED
        );

        for none in [json!({}), json!({"url": ""})] {
            let answer = fetch(As::Admin, none).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(answer.message(), json!("URL was not supplied"));
        }
        for address in [
            local["url"].as_str().expect("an address").to_owned(),
            "http://localhost/".to_owned(),
            "http://192.168.1.1/admin".to_owned(),
            "http://169.254.169.254/latest/meta-data/".to_owned(),
            "http://[::1]:8080/".to_owned(),
            "https://10.0.0.5/".to_owned(),
            "file:///etc/passwd".to_owned(),
        ] {
            let answer = fetch(As::Admin, json!({"url": address})).await;
            assert_eq!(
                answer.status,
                StatusCode::FORBIDDEN,
                "{address}: {}",
                answer.body
            );
            assert_eq!(
                answer.message(),
                json!("Access to local or internal URLs is not allowed")
            );
        }
        assert_eq!(pages.seen().len(), 0);
    })
    .await;
}

#[tokio::test]
async fn page_is_fetched_as_text_as_html_or_as_json_without_a_redirect() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pages = homepage_services::pages().await;
        // The stand-in is on this computer, so the test allows such an address.
        let state = state_with(database, installed_settings(&[])).with_local_pages_scraped();
        let app = router(state).expect("the route table is valid");
        let fetch = async |path: &str, kind: Value| {
            let body = json!({"url": format!("{}{path}", pages.url()), "type": kind});
            call(&app, As::Admin, "POST", SCRAPE, Some(body)).await
        };

        let answer = fetch("/page", json!("html")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["result"], "Success");
        assert_eq!(data["response_code"], 200);
        assert_eq!(data["full_url"], format!("{}/page", pages.url()));
        assert_eq!(data["drill_url"]["scheme"], "http");
        assert_eq!(data["drill_url"]["host"], "127.0.0.1");
        assert_eq!(data["drill_url"]["path"], "/page");
        assert_eq!(data["data"], "<p>Fish & Chips \u{a9} 2026</p>");

        let answer = fetch("/page", Value::Null).await;
        assert_eq!(answer.data()["data"], "<p>Fish &amp; Chips &copy; 2026</p>");
        let answer = fetch("/data.json", json!("json")).await;
        assert_eq!(answer.data()["data"], json!({"answer": 42}));
        let answer = fetch("/page", json!("json")).await;
        assert_eq!(answer.data()["data"], Value::Null);

        // A redirect is not followed, and a page that is not there is an error.
        let before = pages.seen().len();
        let answer = fetch("/moved", json!("html")).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Error getting successful response"));
        assert_eq!(pages.seen().len(), before + 1);
        let answer = fetch("/nowhere", json!("html")).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Error getting successful response"));
    })
    .await;
}
