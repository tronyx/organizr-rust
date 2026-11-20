//! The Tautulli item of the homepage, against a stand-in for Tautulli.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::time::Duration;

use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, JPEG, TAUTULLI_KEY};
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::{installed_app_in, installed_app_with, seed};
use serde_json::{Value, json};

const PATH: &str = "/homepage/tautulli/data";

fn stat_ids(data: &Value) -> Vec<String> {
    data["homestats"]["data"]
        .as_array()
        .expect("a list of statistics")
        .iter()
        .map(|statistic| statistic["stat_id"].as_str().unwrap_or("?").to_owned())
        .collect()
}

#[tokio::test]
async fn statistics_come_for_the_levels_of_the_caller() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::tautulli_statistics().await;
        let app = installed_app_with(
            database,
            &[
                ("homepageTautulliEnabled", json!(true)),
                ("homepageTautulliAuth", json!("4")),
                // A user sees what is watched, and not who watches or the libraries.
                ("homepageTautulliViewsAuth", json!("4")),
                ("tautulliURL", json!(format!("{}/,http://other.example.com", service.url()))),
                ("tautulliApikey", json!(TAUTULLI_KEY)),
                ("homepageTautulliViewingStatsExclude", json!("3")),
                ("homepageTautulliLibraryStatsExclude", json!("3")),
            ],
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let all = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        let data = all.data();
        // The statistics that the homepage never shows are gone.
        assert_eq!(
            stat_ids(&data),
            ["top_movies", "popular_tv", "top_users", "top_platforms"]
        );
        // The film of the library that is left out is gone.
        let films = &data["homestats"]["data"][0]["rows"];
        assert_eq!(films.as_array().map(Vec::len), Some(1));
        assert_eq!(films[0]["title"], "The Film");
        assert_eq!(data["homestats"]["result"], "success");
        assert_eq!(
            data["libstats"]["data"],
            json!([{"section_id": 1, "section_name": "Films", "section_type": "movie", "count": 120, "plays": 300}])
        );
        assert_eq!(
            data["options"],
            json!({
                "url": format!("{}/", service.url()),
                "libraries": true,
                "topMovies": true,
                "topTV": true,
                "topUsers": true,
                "topPlatforms": true,
                "popularMovies": true,
                "popularTV": true,
                "title": true,
                "friendlyName": true,
            })
        );
        let asked: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(
            asked,
            [
                format!("/api/v2?apikey={TAUTULLI_KEY}&cmd=get_home_stats&grouping=1"),
                format!("/api/v2?apikey={TAUTULLI_KEY}&cmd=get_libraries_table"),
            ]
        );

        let some = call(&app, As::User(&user), "GET", PATH, None).await;
        let data = some.data();
        assert_eq!(stat_ids(&data), ["top_movies", "popular_tv"]);
        assert_eq!(data.get("libstats"), None);
        assert_eq!(data["options"]["libraries"], json!(false));
        assert_eq!(data["options"]["topUsers"], json!(false));
        assert_eq!(data["options"]["topMovies"], json!(true));
        // The libraries were not asked for.
        assert_eq!(service.seen().len(), 3);

        // With the levels the other way round, the user sees who watches only.
        let other = installed_app_with(
            database,
            &[
                ("homepageTautulliEnabled", json!(true)),
                ("homepageTautulliAuth", json!("4")),
                ("homepageTautulliMiscAuth", json!("4")),
                ("tautulliURL", json!(service.url())),
                ("tautulliApikey", json!(TAUTULLI_KEY)),
            ],
        );
        let some = call(&other, As::User(&user), "GET", PATH, None).await;
        assert_eq!(stat_ids(&some.data()), ["top_users", "top_platforms"]);
        assert_eq!(some.data()["options"]["topMovies"], json!(false));
        assert_eq!(some.data()["options"]["topUsers"], json!(true));

        let guest = call(&app, As::Guest, "GET", PATH, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None).await.data(),
            json!([{"key": "homepageOrdertautulli", "kind": "tautulli", "id": ""}])
        );
    })
    .await;
}

#[tokio::test]
async fn tautulli_that_refuses_gives_the_options_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::tautulli_statistics().await;
        let settings = |url: String| {
            vec![
                ("homepageTautulliEnabled", json!(true)),
                ("tautulliURL", json!(url)),
                ("tautulliApikey", json!("a-wrong-key")),
            ]
        };

        let wrong = installed_app_with(database, &settings(service.url()));
        let answer = call(&wrong, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data().get("homestats"), None);
        assert_eq!(answer.data()["options"]["title"], json!(true));

        // Nothing listens at this address. The answer names no key.
        let away = installed_app_with(database, &settings("http://127.0.0.1:1".to_owned()));
        let answer = call(&away, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("a-wrong-key"), "{}", answer.body);
    })
    .await;
}

async fn pictures_go_into_the_cache(database: &Database) {
    seed::installed(database).await;
    let service = homepage_services::tautulli_statistics().await;
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let app = installed_app_in(database, folder.path());
    let saved = call(
        &app,
        As::Admin,
        "PUT",
        "/config",
        Some(json!({
            "homepageTautulliEnabled": true,
            "tautulliURL": service.url(),
            "tautulliApikey": TAUTULLI_KEY,
            "cacheImageSize": "1",
        })),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);

    let answer = call(&app, As::Admin, "GET", PATH, None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    let rows = &answer.data()["homestats"]["data"][0]["rows"];
    // The first row of a statistic has its pictures in the cache. The next rows keep
    // the paths of Plex.
    assert_eq!(rows[0]["art"], "data/cache/tautulli-815-np.jpg");
    assert_eq!(rows[0]["thumb"], "data/cache/tautulli-815-list.jpg");
    assert_eq!(rows[1]["thumb"], "/library/metadata/816/thumb/1");

    let address = "/data/cache/tautulli-815-list.jpg";
    let mut status = StatusCode::NOT_FOUND;
    for _ in 0..100 {
        status = send(&app, request("GET", address, &[], "")).await.status;
        if status == StatusCode::OK {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(status, StatusCode::OK);
    let file = folder.path().join("cache").join("tautulli-815-list.jpg");
    assert_eq!(std::fs::read(file).expect("the file is there"), JPEG);
    let asked = service
        .seen()
        .into_iter()
        .find(|seen| seen.path.contains("thumb"))
        .expect("the poster was asked for");
    assert_eq!(
        asked.path,
        format!(
            "/api/v2?apikey={TAUTULLI_KEY}&cmd=pms_image_proxy&img=%2Flibrary%2Fmetadata%2F815%2Fthumb%2F1&rating_key=815&width=200&height=300"
        )
    );
}

#[tokio::test]
async fn pictures_of_the_first_rows_are_fetched_into_the_cache() {
    on_each_engine(pictures_go_into_the_cache).await;
}
