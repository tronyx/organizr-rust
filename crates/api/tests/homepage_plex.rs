//! Plex on the homepage, against a stand-in for a Plex Media Server.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, JPEG, PLEX_SERVER_TOKEN, TAUTULLI_KEY};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_in, installed_app_with, seed};
use serde_json::{Value, json};

const SERVER_ID: &str = "made-up-id-of-the-plex-server";

fn settings(server: &StandIn, more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("homepagePlexEnabled", json!(true)),
        ("homepagePlexAuth", json!("4")),
        ("homepagePlexStreams", json!(true)),
        ("homepagePlexStreamsAuth", json!("4")),
        ("homepagePlexRecent", json!(true)),
        ("homepagePlexPlaylist", json!(true)),
        ("plexURL", json!(server.url())),
        ("plexToken", json!(PLEX_SERVER_TOKEN)),
        ("plexID", json!(SERVER_ID)),
    ];
    settings.extend(more.iter().cloned());
    settings
}

fn titles(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("a list of items")
        .iter()
        .map(|item| item["title"].as_str().unwrap_or("?").to_owned())
        .collect()
}

#[tokio::test]
async fn streams_show_who_plays_only_to_callers_with_the_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = homepage_services::plex_server().await;
        let app = installed_app_with(
            database,
            &settings(
                &server,
                &[
                    ("homepageShowStreamNames", json!(true)),
                    ("homepagePlexStreamsExclude", json!("3")),
                ],
            ),
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let path = "/homepage/plex/streams";

        let all = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        let data = all.data();
        assert_eq!(data["plexID"], SERVER_ID);
        assert_eq!(data["showNames"], json!(true));
        assert_eq!(data["group"], "1");
        // The film of the library that is left out is gone.
        assert_eq!(titles(&data["content"]), ["The Show"]);
        let stream = &data["content"][0];
        assert_eq!(stream["user"], "robin");
        assert_eq!(stream["userAddress"], "192.0.2.50");
        assert_eq!(stream["watched"], 25);
        assert_eq!(stream["nowPlayingKey"], "30-np");
        let seen = &server.seen()[0];
        assert_eq!(seen.path, "/status/sessions");
        assert_eq!(seen.header("x-plex-token"), Some(PLEX_SERVER_TOKEN));

        // The names are for Admins and Co-Admins.
        let some = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(some.data()["content"][0]["user"], "");
        assert_eq!(some.data()["content"][0]["userAddress"], "x.x.x.x");

        // Another setting shows a user the names, and not the addresses.
        let names_only = installed_app_with(
            database,
            &settings(
                &server,
                &[
                    ("homepageShowStreamNamesWithoutIp", json!(true)),
                    ("homepageShowStreamNamesWithoutIpAuth", json!("4")),
                ],
            ),
        );
        let some = call(&names_only, As::User(&user), "GET", path, None).await;
        assert_eq!(some.data()["content"][0]["user"], "robin");
        assert_eq!(some.data()["content"][0]["userAddress"], "x.x.x.x");

        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrderplexnowplaying", "kind": "plexstreams", "id": ""}])
        );
    })
    .await;
}

#[tokio::test]
async fn streams_use_the_names_of_tautulli_when_the_settings_say_so() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = homepage_services::plex_server().await;
        let tautulli = homepage_services::tautulli_statistics().await;
        let app = installed_app_with(
            database,
            &settings(
                &server,
                &[
                    ("homepageShowStreamNames", json!(true)),
                    ("homepageUseCustomStreamNames", json!(true)),
                    ("tautulliURL", json!(tautulli.url())),
                    ("tautulliApikey", json!(TAUTULLI_KEY)),
                ],
            ),
        );

        let answer = call(&app, As::Admin, "GET", "/homepage/plex/streams", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let users: Vec<String> = answer.data()["content"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|item| item["user"].as_str().unwrap_or("?").to_owned())
            .collect();
        // Tautulli has another name for robin, and none for pat.
        assert_eq!(users, ["Robin R.", "pat"]);

        let names = call(&app, As::Admin, "GET", "/homepage/tautulli/names", None).await;
        assert_eq!(names.data(), json!({"robin": "Robin R."}));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let refused = call(
            &app,
            As::User(&user),
            "GET",
            "/homepage/tautulli/names",
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn recent_items_come_newest_first_and_playlists_with_their_videos() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = homepage_services::plex_server().await;
        let app = installed_app_with(
            database,
            &settings(&server, &[("homepageRecentLimit", json!("5"))]),
        );

        let recent = call(&app, As::Admin, "GET", "/homepage/plex/recent", None).await;
        assert_eq!(recent.status, StatusCode::OK, "{}", recent.body);
        // The season was added after the film.
        assert_eq!(titles(&recent.data()["content"]), ["The Show", "The Film"]);
        let asked: Vec<String> = server.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(
            asked,
            [
                "/hubs/home/recentlyAdded?X-Plex-Container-Start=0&X-Plex-Container-Size=5&type=1",
                "/hubs/home/recentlyAdded?X-Plex-Container-Start=0&X-Plex-Container-Size=5&type=2",
                "/hubs/home/recentlyAdded?X-Plex-Container-Start=0&X-Plex-Container-Size=5&type=8",
            ]
        );

        let lists = call(&app, As::Admin, "GET", "/homepage/plex/playlists", None).await;
        assert_eq!(lists.status, StatusCode::OK, "{}", lists.body);
        let content = &lists.data()["content"];
        // Only the video playlist without "private" in its title.
        assert_eq!(
            content
                .as_object()
                .map(|lists| lists.keys().cloned().collect::<Vec<_>>()),
            Some(vec!["Filmnight".to_owned()])
        );
        assert_eq!(content["Filmnight"]["title"], "Film night!");
        assert_eq!(content["Filmnight"]["0"]["title"], "The Film");
        // The playlist whose path is no path of the playlists was not asked for.
        assert!(
            !server
                .seen()
                .iter()
                .any(|seen| seen.path == "/status/sessions")
        );

        // A user has no level for these two parts.
        let user = seed::logged_in(database, &app, "zed", 4).await;
        for path in ["/homepage/plex/recent", "/homepage/plex/playlists"] {
            let refused = call(&app, As::User(&user), "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
        }
    })
    .await;
}

#[tokio::test]
async fn details_and_search_give_items_and_a_wrong_token_is_reported() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = homepage_services::plex_server().await;
        let app = installed_app_with(
            database,
            &settings(&server, &[("mediaSearch", json!(true))]),
        );

        let details = call(
            &app,
            As::Admin,
            "POST",
            "/homepage/plex/metadata",
            Some(json!({"key": "815"})),
        )
        .await;
        assert_eq!(details.status, StatusCode::OK, "{}", details.body);
        let film = &details.data()["content"][0];
        assert_eq!(film["title"], "The Film");
        assert_eq!(film["metadata"]["genres"], json!(["Drama"]));
        assert_eq!(server.seen()[0].path, "/library/metadata/815");

        // A key that is no number could name another path of the server.
        for key in [json!("815/../../status/sessions"), json!(""), Value::Null] {
            let refused = call(
                &app,
                As::Admin,
                "POST",
                "/homepage/plex/metadata",
                Some(json!({"key": key})),
            )
            .await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(refused.message(), json!("Plex Metadata key is not defined"));
        }
        assert_eq!(server.seen().len(), 1);

        let found = call(
            &app,
            As::Admin,
            "GET",
            "/homepage/plex/search/the%20film",
            None,
        )
        .await;
        assert_eq!(found.status, StatusCode::OK, "{}", found.body);
        // The episode is left out.
        assert_eq!(titles(&found.data()["content"]), ["The Film"]);
        assert_eq!(
            server.seen().last().map(|seen| seen.path.clone()),
            Some("/search?query=the+film".to_owned())
        );

        let wrong = installed_app_with(
            database,
            &settings(&server, &[("plexToken", json!("another-token"))]),
        );
        let refused = call(&wrong, As::Admin, "GET", "/homepage/plex/streams", None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(refused.data(), json!([]));
        let refused = call(&wrong, As::Admin, "GET", "/homepage/plex/playlists", None).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(refused.message(), json!("Plex API error"));
    })
    .await;
}

async fn picture(app: &Router, who: As<'_>, path: &str) -> Answer {
    call(app, who, "GET", path, None).await
}

async fn pictures_are_fetched_and_kept(database: &Database) {
    seed::installed(database).await;
    let server = homepage_services::plex_server().await;
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let app = installed_app_in(database, folder.path());
    let saved = call(
        &app,
        As::Admin,
        "PUT",
        "/config",
        Some(json!({
            "homepagePlexEnabled": true,
            "plexURL": server.url(),
            "plexToken": PLEX_SERVER_TOKEN,
            "plexID": SERVER_ID,
        })),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let path = "/homepage/image?source=plex&img=/library/metadata/815/thumb/1&height=300&width=200&key=815-list";

    let first = picture(&app, As::Admin, path).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let header = |answer: &Answer, name: &str| {
        answer
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    assert_eq!(
        header(&first, "content-type").as_deref(),
        Some("image/jpeg")
    );
    assert_eq!(
        header(&first, "x-content-type-options").as_deref(),
        Some("nosniff")
    );
    let seen = server.seen();
    assert_eq!(
        seen[0].path,
        "/photo/:/transcode?height=300&width=200&upscale=1&url=%2Flibrary%2Fmetadata%2F815%2Fthumb%2F1"
    );
    assert_eq!(seen[0].header("x-plex-token"), Some(PLEX_SERVER_TOKEN));
    let file = folder.path().join("cache").join("plex-815-list.jpg");
    assert_eq!(std::fs::read(file).expect("the file is there"), JPEG);

    // The second time, the picture comes from the cache. A `$` asks for it anew.
    picture(&app, As::Admin, path).await;
    assert_eq!(server.seen().len(), 1);
    let again = picture(&app, As::Admin, &format!("{path}$abc")).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(server.seen().len(), 2);

    // A guest has no level for Plex. An address that is no path of the server, a size
    // that is too large and a key that could leave the folder are refused.
    let refused = picture(&app, As::Guest, path).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    for wrong in [
        "/homepage/image?source=plex&img=https://evil.example.net/a.jpg&height=300&width=200&key=1-list",
        "/homepage/image?source=plex&img=//evil.example.net/a.jpg&height=300&width=200&key=1-list",
        "/homepage/image?source=plex&img=/library/metadata/1/thumb/1&height=30000&width=200&key=1-list",
        "/homepage/image?source=plex&img=/library/metadata/1/thumb/1&height=300&width=200&key=..%2Fsecret",
        "/homepage/image?source=emby&img=/library/metadata/1/thumb/1&height=300&width=200&key=1-list",
    ] {
        let refused = picture(&app, As::Admin, wrong).await;
        assert_eq!(refused.status, StatusCode::NOT_FOUND, "{wrong}");
    }
    assert_eq!(server.seen().len(), 2);
}

#[tokio::test]
async fn pictures_of_the_server_are_fetched_in_a_size_and_kept() {
    on_each_engine(pictures_are_fetched_and_kept).await;
}
