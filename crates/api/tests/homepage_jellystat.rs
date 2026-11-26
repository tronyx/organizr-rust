//! The JellyStat item of the homepage and its connection test, against stand-ins for
//! JellyStat and for a Jellyfin server.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, JELLYSTAT_KEY, MEDIA_SERVER_KEY};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const DATA: &str = "/homepage/jellystat";
const DETAILS: &str = "/homepage/jellystat/metadata";
const TEST: &str = "/test/jellystat";

type Changed = Vec<(&'static str, Value)>;

fn settings(url: &str) -> Changed {
    vec![
        ("homepageJellyStatEnabled", json!(true)),
        ("homepageJellyStatAuth", json!("4")),
        ("jellyStatURL", json!(url)),
        ("jellyStatApikey", json!(JELLYSTAT_KEY)),
    ]
}

/// The settings with another value for one of them.
fn with(mut settings: Changed, key: &'static str, value: Value) -> Changed {
    settings.retain(|(known, _)| *known != key);
    settings.push((key, value));
    settings
}

#[tokio::test]
async fn statistics_have_the_libraries_the_users_and_the_most_played_items() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jellystat().await;
        let app = installed_app_with(database, &settings(&service.url()));

        let answer = call(&app, As::Admin, "GET", DATA, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("JellyStat data retrieved successfully")
        );
        let data = answer.data();
        assert_eq!(data["period"], "30 days");
        assert_eq!(data["libraries"][0]["name"], "Films");
        assert_eq!(data["libraries"][0]["type"], "Movies");
        assert_eq!(data["libraries"][0]["total_play_time"], "01:01:01");
        assert_eq!(data["libraries"][1]["total_play_time"], "0 min");
        assert_eq!(data["library_totals"]["total_libraries"], 2);
        assert_eq!(data["library_totals"]["total_items"], 160);
        assert_eq!(data["library_totals"]["total_episodes"], 800);
        assert_eq!(
            data["library_totals"]["type_breakdown"]["tvshows"]["label"],
            "TV Shows"
        );
        assert_eq!(data["server_info"]["server_id"], "made-up-server-id");
        assert_eq!(
            data["most_watched_movies"],
            json!([{
                "id": "815", "title": "The Film", "type": "movie", "play_count": 1,
                "total_duration": 5400, "year": "2019", "server_id": "made-up-server-id",
                "poster_path": null, "first_played": "2026-11-18T20:00:00.000Z",
                "last_played": "2026-11-18T20:00:00.000Z",
            }])
        );
        assert_eq!(data["most_watched_shows"][0]["title"], "The Show");
        assert_eq!(data["most_watched_shows"][0]["play_count"], 2);
        assert_eq!(data["most_watched_shows"][0]["id"], "301");
        assert_eq!(data["most_listened_music"][0]["title"], "A Song");
        assert_eq!(
            data["users"],
            json!([
                {"name": "robin", "play_count": 2, "last_activity": "2026-11-19T20:00:00.000Z"},
                {"name": "pat", "play_count": 2, "last_activity": "2026-11-17T20:00:00.000Z"},
            ])
        );

        // The key goes in a header, and the history is asked page by page.
        let seen = service.seen();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].path, "/api/getLibraries");
        assert_eq!(seen[0].header("x-api-token"), Some(JELLYSTAT_KEY));
        assert!(
            seen[1]
                .path
                .starts_with("/api/getHistory?page=1&size=1000&startDate=20"),
            "{}",
            seen[1].path
        );
        assert!(
            seen.iter()
                .all(|request| !request.path.contains(JELLYSTAT_KEY))
        );

        // A list that is switched off is empty.
        let fewer = with(
            settings(&service.url()),
            "homepageJellyStatShowMostWatchedMovies",
            json!(false),
        );
        let answer = call(
            &installed_app_with(database, &fewer),
            As::Admin,
            "GET",
            DATA,
            None,
        )
        .await;
        assert_eq!(answer.data()["most_watched_movies"], json!([]));
        assert_eq!(
            answer.data()["most_watched_shows"].as_array().map(Vec::len),
            Some(1)
        );

        // The item shows, with its options and without the key.
        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(items.data()[0]["key"], "homepageOrderJellyStat");
        assert_eq!(items.data()[0]["kind"], "jellystat");
        assert_eq!(items.data()[0]["options"]["mode"], "native");
        assert_eq!(items.data()[0]["options"]["url"], service.url());
        assert_eq!(items.data()[0]["options"]["refreshMinutes"], 5);
        assert!(!items.body.contains(JELLYSTAT_KEY), "{}", items.body);
    })
    .await;
}

#[tokio::test]
async fn statistics_are_refused_with_one_text_and_fail_without_the_key() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jellystat().await;
        let url = service.url();
        let refusal =
            json!("User not approved to view this homepage item - check plugin configuration");

        let off = with(settings(&url), "homepageJellyStatEnabled", json!(false));
        let answer = call(
            &installed_app_with(database, &off),
            As::Admin,
            "GET",
            DATA,
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), refusal);

        let on = installed_app_with(database, &settings(&url));
        let answer = call(&on, As::Guest, "GET", DATA, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), refusal);
        let user = seed::logged_in(database, &on, "uma", 4).await;
        let answer = call(&on, As::User(&user), "GET", DATA, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let no_key = with(settings(&url), "jellyStatApikey", json!(""));
        let answer = call(
            &installed_app_with(database, &no_key),
            As::Admin,
            "GET",
            DATA,
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), refusal);

        // The item with the frame needs no key, and has no statistics without one.
        let mut frame = with(no_key, "homepageJellyStatDisplayMode", json!("iframe"));
        frame.push(("homepageJellyStatIframeHeight", json!("600")));
        let framed = installed_app_with(database, &frame);
        let answer = call(&framed, As::Admin, "GET", DATA, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            answer.message(),
            json!("JellyStat URL or API key not configured")
        );
        let items = call(&framed, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(items.data()[0]["options"]["mode"], "iframe");
        assert_eq!(items.data()[0]["options"]["frameHeight"], 600);

        // JellyStat refuses the key: every list is empty.
        let wrong = with(settings(&url), "jellyStatApikey", json!("a-wrong-key"));
        let answer = call(
            &installed_app_with(database, &wrong),
            As::Admin,
            "GET",
            DATA,
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["libraries"], json!([]));
        assert_eq!(answer.data()["library_totals"], json!([]));
        assert_eq!(answer.data()["users"], json!([]));

        // Nothing listens at this address.
        let away = installed_app_with(database, &settings("http://127.0.0.1:1"));
        let answer = call(&away, As::Admin, "GET", DATA, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            answer
                .message()
                .as_str()
                .is_some_and(|text| text.starts_with("Failed to fetch JellyStat data: ")),
            "{}",
            answer.body
        );
        assert!(!answer.body.contains(JELLYSTAT_KEY), "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn details_of_an_item_come_from_the_media_server_or_from_jellystat() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jellystat().await;
        let jellyfin = homepage_services::jellyfin().await;
        let key = |key: &str| Some(json!({ "key": key }));

        // Without a media server, JellyStat is asked.
        let alone = installed_app_with(database, &settings(&service.url()));
        let answer = call(&alone, As::Admin, "POST", DETAILS, key("815")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = answer.data()["content"].clone();
        assert_eq!(content.as_array().map(Vec::len), Some(1));
        assert_eq!(content[0]["title"], "The Film");
        assert_eq!(content[0]["uid"], "815");
        assert_eq!(content[0]["key"], "815-list");
        assert_eq!(content[0]["metadata"]["summary"], "A film, as JellyStat knows it.");
        assert_eq!(
            content[0]["imageURL"],
            format!("{}/proxy/Items/815/Images/Primary?tag=made-up-tag&fillWidth=400&quality=90", service.url())
        );
        assert_eq!(content[0]["address"], "");
        assert_eq!(content[0]["openTab"], false);
        assert_eq!(service.seen()[0].path, "/api/getItem?id=815");
        assert_eq!(service.seen()[0].header("x-api-token"), Some(JELLYSTAT_KEY));

        // An item that nobody knows still has an answer.
        let answer = call(&alone, As::Admin, "POST", DETAILS, key("999")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()["content"][0]["title"], "Unknown Item");
        assert_eq!(answer.data()["content"][0]["metadata"]["tagline"], "View in JellyStat");

        // With Jellyfin on the homepage, Jellyfin is asked, and its key stays here.
        let mut both = settings(&service.url());
        both.extend([
            ("homepageJellyfinEnabled", json!(true)),
            ("jellyfinURL", json!(jellyfin.url())),
            ("jellyfinToken", json!(MEDIA_SERVER_KEY)),
            ("jellyfinTabName", json!("Jellyfin")),
            ("homepageJellyfinLink", json!("https://media.example.com/#!/details?id={id}&serverId={serverId}")),
        ]);
        let with_server = installed_app_with(database, &both);
        let before = service.seen().len();
        let answer = call(&with_server, As::Admin, "POST", DETAILS, key("815")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let item = answer.data()["content"][0].clone();
        assert_eq!(item["title"], "The Film");
        assert_eq!(item["type"], "movie");
        assert_eq!(item["metadata"]["summary"], "A film.");
        assert_eq!(item["metadata"]["year"], "2019");
        assert_eq!(item["metadata"]["rating"], "7.5");
        assert_eq!(item["metadata"]["duration"], "6000000");
        assert_eq!(item["metadata"]["actors"][0]["name"], "An Actor");
        assert_eq!(
            item["imageURL"],
            "api/v2/homepage/image?source=jellyfin&type=Primary&img=815&height=600&width=400&key=815-jellystat"
        );
        assert_eq!(
            item["address"],
            "https://media.example.com/#!/details?id=815&serverId=made-up-server-id"
        );
        assert_eq!(item["tabName"], "Jellyfin");
        assert_eq!(item["openTab"], true);
        assert!(!answer.body.contains(MEDIA_SERVER_KEY), "{}", answer.body);
        assert!(!answer.body.contains("api_key"), "{}", answer.body);
        assert_eq!(service.seen().len(), before);

        // The key must be an id, and the caller needs the level of the item.
        for wrong in [json!({"key": "../Users"}), json!({"key": ""}), json!({})] {
            let answer = call(&alone, As::Admin, "POST", DETAILS, Some(wrong)).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(answer.message(), json!("JellyStat metadata key is not defined"));
        }
        let answer = call(&alone, As::Guest, "POST", DETAILS, key("815")).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), json!("Not authorized for JellyStat metadata"));
    })
    .await;
}

#[tokio::test]
async fn media_server_is_asked_only_for_a_caller_with_its_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jellystat().await;
        let jellyfin = homepage_services::jellyfin().await;
        let mut changed = with(
            settings(&service.url()),
            "homepageJellyStatAuth",
            json!("999"),
        );
        changed.extend([
            ("homepageJellyfinEnabled", json!(true)),
            ("homepageJellyfinAuth", json!("1")),
            ("jellyfinURL", json!(jellyfin.url())),
            ("jellyfinToken", json!(MEDIA_SERVER_KEY)),
            ("jellyfinTabName", json!("Jellyfin")),
        ]);
        let app = installed_app_with(database, &changed);

        // A guest sees JellyStat, but not the Jellyfin item: JellyStat is asked.
        let answer = call(
            &app,
            As::Guest,
            "POST",
            DETAILS,
            Some(json!({"key": "815"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let item = answer.data()["content"][0].clone();
        assert_eq!(
            item["metadata"]["summary"],
            "A film, as JellyStat knows it."
        );
        assert_eq!(item["openTab"], false);
        assert!(jellyfin.seen().is_empty(), "Jellyfin was not asked");
        assert_eq!(service.seen().len(), 1);

        // The Admin has the level of the Jellyfin item.
        let answer = call(
            &app,
            As::Admin,
            "POST",
            DETAILS,
            Some(json!({"key": "815"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data()["content"][0]["metadata"]["summary"],
            "A film."
        );
        assert_eq!(service.seen().len(), 1);
    })
    .await;
}

#[tokio::test]
async fn connection_test_asks_for_the_libraries_or_for_the_page() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jellystat().await;
        let url = service.url();
        let test = async |changed: &Changed| {
            call(&installed_app_with(database, changed), As::Admin, "POST", TEST, None).await
        };

        let answer = test(&settings(&url)).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Successfully connected to JellyStat API"));

        let wrong = test(&with(settings(&url), "jellyStatApikey", json!("a-wrong-key"))).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!("JellyStat is reachable but API key is invalid or API endpoints are not responding correctly.")
        );

        let frame = with(settings(&url), "homepageJellyStatDisplayMode", json!("iframe"));
        let answer = test(&frame).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Successfully connected to JellyStat"));

        let away = test(&settings("http://127.0.0.1:1")).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            away.message().as_str().is_some_and(|text| text.starts_with("Connection test failed: ")),
            "{}",
            away.body
        );
        assert!(!away.body.contains(JELLYSTAT_KEY), "{}", away.body);

        let off = test(&with(settings(&url), "homepageJellyStatEnabled", json!(false))).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(off.message(), json!("homepageJellyStatEnabled module is not enabled"));
        let no_key = test(&with(settings(&url), "jellyStatApikey", json!(""))).await;
        assert_eq!(no_key.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(no_key.message(), json!("jellyStatApikey was not supplied"));

        // A user of the item cannot run the test.
        let app = installed_app_with(database, &settings(&url));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(&app, As::User(&user), "POST", TEST, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(call(&app, As::Guest, "POST", TEST, None).await.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}
