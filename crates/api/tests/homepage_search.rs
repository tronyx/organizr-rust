//! The Jackett item and the Prowlarr item of the homepage, and their connection tests,
//! against stand-ins for the two services.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, JACKETT_KEY, PROWLARR_GUID, PROWLARR_KEY};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const NOT_APPROVED: &str = "User not approved to view this homepage item";

fn jackett_settings(url: &str, key: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("homepageJackettEnabled", json!(true)),
        ("jackettURL", json!(url)),
        ("jackettToken", json!(key)),
    ]
}

fn prowlarr_settings(url: &str, key: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("homepageProwlarrEnabled", json!(true)),
        ("prowlarrURL", json!(url)),
        ("prowlarrToken", json!(key)),
    ]
}

/// The item of a kind in the list of the homepage, when the caller sees it.
fn listed(items: &Answer, kind: &str) -> Option<Value> {
    items
        .data()
        .as_array()
        .expect("the items are a list")
        .iter()
        .find(|item| item["kind"] == json!(kind))
        .cloned()
}

#[tokio::test]
async fn jackett_search_gives_the_releases_without_the_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jackett().await;
        let app = installed_app_with(database, &jackett_settings(&service.url(), JACKETT_KEY));

        let answer = call(&app, As::Admin, "GET", "/homepage/jackett/A%20Film", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let results = &answer.data()["content"]["Results"];
        assert_eq!(results.as_array().map(Vec::len), Some(2));
        assert_eq!(results[0]["Title"], json!("A Film 2024 1080p"));
        assert_eq!(results[0]["Seeders"], json!(45));
        assert_eq!(
            results[0]["BlackholeLink"],
            json!(
                "http://127.0.0.1:9117/bh/tracker/?jackett_apikey=&path=made-up-path&file=A+Film+2024"
            )
        );
        // The key of Jackett stays on the server.
        assert!(!answer.body.contains(JACKETT_KEY), "{}", answer.body);
        assert_eq!(
            service.seen()[0].path,
            format!("/api/v2.0/indexers/all/results?apikey={JACKETT_KEY}&Query=A+Film")
        );

        // The list of the homepage names the item, and how a release is downloaded.
        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        let item = listed(&items, "jackett").expect("the item is listed");
        assert_eq!(item["key"], json!("homepageOrderJackett"));
        assert_eq!(item["options"], json!({"blackHole": false}));

        // Jackett refuses a wrong key. The answer then has no content, as in PHP.
        let app = installed_app_with(database, &jackett_settings(&service.url(), "wrong"));
        let answer = call(&app, As::Admin, "GET", "/homepage/jackett/film", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!({"content": false}));
    })
    .await;
}

#[tokio::test]
async fn jackett_download_asks_for_a_black_hole_link_with_the_stored_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jackett().await;
        let mut settings = jackett_settings(&service.url(), JACKETT_KEY);
        settings.push(("homepageJackettBackholeDownload", json!(true)));
        let app = installed_app_with(database, &settings);
        let download = async |body: Value| {
            call(
                &app,
                As::Admin,
                "POST",
                "/homepage/jackett/download/",
                Some(body),
            )
            .await
        };

        // The link comes as the search gave it: without the key.
        let link = "/bh/tracker/?jackett_apikey=&path=made-up-path&file=A+Film+2024";
        let answer = download(json!({ "url": link })).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), json!({"content": {"result": "success"}}));
        assert_eq!(
            service.seen().last().expect("a request came").path,
            format!("/bh/tracker/?jackett_apikey={JACKETT_KEY}&path=made-up-path&file=A+Film+2024")
        );

        // An error of Jackett is the message of the answer.
        let answer = download(json!({"url": "/bh/tracker/?path=other"})).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            answer.message(),
            json!("Blackhole directory does not exist")
        );
        assert_eq!(answer.data()["content"]["result"], json!("error"));

        // Without a link, and with a link that is no black hole link, Jackett is not
        // asked.
        let asked = service.seen().len();
        let answer = download(json!({})).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("URL was not supplied"));
        for link in [
            "/api/v2.0/server/config",
            "@other.example.com/bh/tracker/",
            "/bh/../api/v2.0/server/config",
        ] {
            let answer = download(json!({ "url": link })).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{link}");
            assert_eq!(
                answer.message(),
                json!("URL is not a black hole link of Jackett")
            );
        }
        assert_eq!(service.seen().len(), asked);

        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        let item = listed(&items, "jackett").expect("the item is listed");
        assert_eq!(item["options"], json!({"blackHole": true}));
    })
    .await;
}

#[tokio::test]
async fn jackett_is_refused_as_every_item_and_its_errors_name_no_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let secret = "made-up-secret-key";
        let settings = jackett_settings("http://127.0.0.1:9", secret);
        let app = installed_app_with(database, &settings);
        let search = "/homepage/jackett/film";
        let download = "/homepage/jackett/download/";
        let body = || Some(json!({"url": "/bh/tracker/?path=made-up-path"}));

        // A service that cannot be reached.
        for answer in [
            call(&app, As::Admin, "GET", search, None).await,
            call(&app, As::Admin, "POST", download, body()).await,
            call(&app, As::Admin, "POST", "/test/jackett", None).await,
        ] {
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
            assert!(!answer.body.contains(secret), "{}", answer.body);
            assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        }

        // The level of the item is the one of a Co-Admin, when the settings name none.
        let user = seed::logged_in(database, &app, "Robin", 4).await;
        for who in [As::Guest, As::User(&user)] {
            let answer = call(&app, who, "GET", search, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            assert_eq!(answer.message(), json!(NOT_APPROVED));
            let answer = call(&app, who, "POST", download, body()).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            let answer = call(&app, who, "POST", "/test/jackett", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            let items = call(&app, who, "GET", "/homepage/items", None).await;
            assert_eq!(listed(&items, "jackett"), None);
        }

        // The switch is off.
        let off = installed_app_with(database, &settings[1..]);
        let answer = call(&off, As::Admin, "GET", search, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepageJackettEnabled module is not enabled")
        );
        let answer = call(&off, As::Admin, "POST", download, body()).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        let items = call(&off, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(listed(&items, "jackett"), None);

        // The address or the key is missing. The test does not need the switch.
        let no_key = installed_app_with(database, &settings[..2]);
        for (method, path) in [("GET", search), ("POST", "/test/jackett")] {
            let answer = call(&no_key, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
            assert_eq!(answer.message(), json!("jackettToken was not supplied"));
        }
        let no_address = installed_app_with(database, &[settings[0].clone(), settings[2].clone()]);
        let answer = call(&no_address, As::Admin, "GET", search, None).await;
        assert_eq!(answer.message(), json!("jackettURL was not supplied"));
    })
    .await;
}

#[tokio::test]
async fn prowlarr_search_gives_the_releases_without_the_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::prowlarr().await;
        let app = installed_app_with(database, &prowlarr_settings(&service.url(), PROWLARR_KEY));

        let answer = call(&app, As::Admin, "GET", "/homepage/prowlarr/A%20Film", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content.as_array().map(Vec::len), Some(1));
        assert_eq!(content[0]["title"], json!("A Film 2024 1080p"));
        assert_eq!(content[0]["indexerId"], json!(5));
        assert_eq!(
            content[0]["downloadUrl"],
            json!("http://127.0.0.1:9696/5/download?apikey=&link=made-up-link&file=A+Film+2024")
        );
        assert!(!answer.body.contains(PROWLARR_KEY), "{}", answer.body);
        // The key goes in a header, and not in the address.
        let seen = &service.seen()[0];
        assert_eq!(seen.path, "/api/v1/search?query=A+Film");
        assert_eq!(seen.header("x-api-key"), Some(PROWLARR_KEY));

        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        let item = listed(&items, "prowlarr").expect("the item is listed");
        assert_eq!(item["key"], json!("homepageOrderProwlarr"));
        assert_eq!(item["options"], json!({"blackHole": false}));

        let app = installed_app_with(database, &prowlarr_settings(&service.url(), "wrong"));
        let answer = call(&app, As::Admin, "GET", "/homepage/prowlarr/film", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!({"content": false}));
    })
    .await;
}

#[tokio::test]
async fn prowlarr_download_hands_a_release_to_prowlarr() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::prowlarr().await;
        let app = installed_app_with(database, &prowlarr_settings(&service.url(), PROWLARR_KEY));
        let download = async |body: Value| {
            call(
                &app,
                As::Admin,
                "POST",
                "/homepage/prowlarr/download/",
                Some(body),
            )
            .await
        };

        // The interface sends the id of the indexer as text.
        let answer = download(json!({"guid": PROWLARR_GUID, "indexerId": "5"})).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = answer.data()["content"].clone();
        let text = content.as_str().expect("the content is a text");
        assert!(text.contains("A Film 2024 1080p"), "{text}");
        assert!(!answer.body.contains(PROWLARR_KEY), "{}", answer.body);
        let seen = service.seen().last().expect("a request came").clone();
        assert_eq!(seen.method, "POST");
        assert_eq!(seen.path, "/api/v1/search");
        assert_eq!(seen.header("x-api-key"), Some(PROWLARR_KEY));
        assert_eq!(seen.json(), json!({"guid": PROWLARR_GUID, "indexerId": 5}));

        // Prowlarr does not know the release.
        let answer = download(json!({"guid": "unknown", "indexerId": 5})).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST);
        assert_eq!(answer.message(), json!("Unknown error"));
        assert_eq!(answer.data(), json!({"content": false}));

        let asked = service.seen().len();
        let answer = download(json!({"indexerId": 5})).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("guid was not supplied"));
        for body in [
            json!({"guid": PROWLARR_GUID}),
            json!({"guid": PROWLARR_GUID, "indexerId": "five"}),
        ] {
            let answer = download(body).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(answer.message(), json!("indexerId was not supplied"));
        }
        assert_eq!(service.seen().len(), asked);
    })
    .await;
}

#[tokio::test]
async fn prowlarr_is_refused_as_every_item_with_the_texts_of_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let secret = "made-up-secret-key";
        let settings = prowlarr_settings("http://127.0.0.1:9", secret);
        let app = installed_app_with(database, &settings);
        let search = "/homepage/prowlarr/film";
        let download = "/homepage/prowlarr/download/";
        let body = || Some(json!({"guid": PROWLARR_GUID, "indexerId": 5}));

        for answer in [
            call(&app, As::Admin, "GET", search, None).await,
            call(&app, As::Admin, "POST", download, body()).await,
            call(&app, As::Admin, "POST", "/test/prowlarr", None).await,
        ] {
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
            assert!(!answer.body.contains(secret), "{}", answer.body);
            assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        }

        let user = seed::logged_in(database, &app, "Robin", 4).await;
        for who in [As::Guest, As::User(&user)] {
            let answer = call(&app, who, "GET", search, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            assert_eq!(answer.message(), json!(NOT_APPROVED));
            let answer = call(&app, who, "POST", download, body()).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            let answer = call(&app, who, "POST", "/test/prowlarr", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        }

        let off = installed_app_with(database, &settings[1..]);
        let answer = call(&off, As::Admin, "GET", search, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepageProwlarrEnabled module is not enabled")
        );
        let items = call(&off, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(listed(&items, "prowlarr"), None);

        // PHP names the two settings with a small letter at the start.
        let no_key = installed_app_with(database, &settings[..2]);
        for (method, path) in [("GET", search), ("POST", "/test/prowlarr")] {
            let answer = call(&no_key, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{path}");
            assert_eq!(answer.message(), json!("prowlarrToken was not supplied"));
        }
        let no_address = installed_app_with(database, &[settings[0].clone(), settings[2].clone()]);
        let answer = call(&no_address, As::Admin, "POST", download, body()).await;
        assert_eq!(answer.message(), json!("prowlarrURL was not supplied"));
    })
    .await;
}

#[tokio::test]
async fn tests_of_jackett_and_prowlarr_search_and_show_what_was_found() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let jackett = homepage_services::jackett().await;
        // The test does not need the switch of the item.
        let settings = |key: &str| jackett_settings(&jackett.url(), key)[1..].to_vec();
        let app = installed_app_with(database, &settings(JACKETT_KEY));
        let answer = call(&app, As::Admin, "POST", "/test/jackett", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["content"]["Results"], json!([]));
        assert!(
            jackett.seen()[0]
                .path
                .ends_with("&Query=this-is-just-a-test-for-organizr")
        );
        let app = installed_app_with(database, &settings("wrong"));
        let answer = call(&app, As::Admin, "POST", "/test/jackett", None).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        assert_eq!(answer.message(), json!("Error connecting to Jackett"));

        let prowlarr = homepage_services::prowlarr().await;
        let settings = |key: &str| prowlarr_settings(&prowlarr.url(), key)[1..].to_vec();
        let app = installed_app_with(database, &settings(PROWLARR_KEY));
        let answer = call(&app, As::Admin, "POST", "/test/prowlarr", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), json!({"content": []}));
        let app = installed_app_with(database, &settings("wrong"));
        let answer = call(&app, As::Admin, "POST", "/test/prowlarr", None).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        assert_eq!(answer.message(), json!("Error connecting to Prowlarr"));
    })
    .await;
}
