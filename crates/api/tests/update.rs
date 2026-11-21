//! The routes `/update/*`: the version check against the releases of the project, the
//! answers `409` of the routes that would update the install by a script, and the
//! reset of a feature (D-23).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The text of the routes that would update the install by a script.
const PULL_THE_IMAGE: &str =
    "This install runs from a container image. To update, pull the new image and start it again";

/// The application, with the releases API at the given address.
fn app_with_releases(database: &Database, releases: &str) -> Router {
    let state = state_with(database, installed_settings(&[])).with_releases_api(releases);
    organizr_api::router(state).expect("the route table is valid")
}

/// A stand-in for the releases API of GitHub that answers with the given body.
async fn releases(body: &'static str) -> StandIn {
    StandIn::start(Router::new().route(
        "/repos/owner/project/releases/latest",
        get(move || async move { ([("content-type", "application/json")], body) }),
    ))
    .await
}

fn latest_of(stand_in: &StandIn) -> String {
    format!("{}/repos/owner/project/releases/latest", stand_in.url())
}

/// The data of an answer before any check.
fn unchecked() -> Value {
    json!({
        "current": VERSION,
        "latest": null,
        "updateAvailable": false,
        "releaseUrl": null,
    })
}

/// The routes of PHP that update the install by a script, with their texts.
const SCRIPT_ROUTES: [(&str, &str); 9] = [
    ("/update", PULL_THE_IMAGE),
    ("/update/docker", PULL_THE_IMAGE),
    ("/update/download/v2-master", PULL_THE_IMAGE),
    ("/update/unzip/v2-master", PULL_THE_IMAGE),
    ("/update/move/v2-develop", PULL_THE_IMAGE),
    ("/update/cleanup/v2-develop", PULL_THE_IMAGE),
    ("/update/windows", "Your install type is not Windows"),
    ("/update/linux", "Your install type is not Linux"),
    (
        "/update/migrate/2.1.0",
        "The migrations of PHP Organizr do not apply to this install. The import brings the data of an install up to date",
    ),
];

#[tokio::test]
async fn only_admins_and_co_admins_reach_the_update_routes() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = releases(r#"{"tag_name": "v0.0.1"}"#).await;
        let app = app_with_releases(database, &latest_of(&stand_in));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let paths = SCRIPT_ROUTES
            .iter()
            .map(|(path, _)| *path)
            .chain(["/update/check", "/update/reset/groupmax"]);
        for path in paths {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, "GET", path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
                assert_eq!(answer.message(), json!("Not Authorized"), "{path}");
            }
            let answer = call(&app, As::User(&co_admin), "GET", path, None).await;
            assert_ne!(answer.status, StatusCode::UNAUTHORIZED, "{path}");
        }
        // A guest never sees a request to GitHub happen.
        assert_eq!(stand_in.seen().len(), 1, "only the Co-Admin checked");
    })
    .await;
}

#[tokio::test]
async fn script_routes_answer_409_with_the_versions() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = releases(r#"{"tag_name": "v999.1.0", "name": "A release"}"#).await;
        let app = app_with_releases(database, &latest_of(&stand_in));

        for (path, message) in SCRIPT_ROUTES {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{path}");
            assert_eq!(answer.json()["response"]["result"], json!("error"));
            assert_eq!(answer.message(), json!(message), "{path}");
            assert_eq!(answer.data(), unchecked(), "{path}");
        }
        // Nothing went to GitHub.
        assert_eq!(stand_in.seen(), []);

        let checked = call(&app, As::Admin, "GET", "/update/check", None).await;
        assert_eq!(checked.status, StatusCode::OK, "{}", checked.body);
        let found = json!({
            "current": VERSION,
            "latest": "999.1.0",
            "updateAvailable": true,
            "releaseUrl": format!("{}/releases/tag/v999.1.0", env!("CARGO_PKG_REPOSITORY")),
        });
        assert_eq!(checked.data(), found);
        // The routes send the last result.
        for (path, _) in SCRIPT_ROUTES {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(answer.data(), found, "{path}");
        }
    })
    .await;
}

#[tokio::test]
async fn check_compares_the_newest_release_with_this_version() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let same = format!(r#"{{"tag_name": "v{VERSION}"}}"#).leak();
        for (body, latest, newer) in [
            (r#"{"tag_name": "v999.0.0"}"#, "999.0.0", true),
            (&*same, VERSION, false),
            (r#"{"tag_name": "0.0.1"}"#, "0.0.1", false),
        ] {
            let stand_in = releases(body).await;
            let app = app_with_releases(database, &latest_of(&stand_in));
            let answer = call(&app, As::Admin, "GET", "/update/check", None).await;
            assert_eq!(answer.status, StatusCode::OK, "{body}: {}", answer.body);
            assert_eq!(answer.data()["latest"], json!(latest), "{body}");
            assert_eq!(answer.data()["updateAvailable"], json!(newer), "{body}");

            // The request is small: no token, and a name of the application only.
            let seen = stand_in.seen();
            assert_eq!(seen.len(), 1);
            assert_eq!(seen[0].method, "GET");
            assert_eq!(seen[0].header("user-agent"), Some("organizr"));
            assert_eq!(seen[0].header("authorization"), None);
            assert_eq!(seen[0].header("cookie"), None);

            // An Admin sees the result in the launch answer, a guest does not.
            let launch = call(&app, As::Admin, "GET", "/launch", None).await;
            assert_eq!(launch.data()["update"], answer.data(), "{body}");
            let launch = call(&app, As::Guest, "GET", "/launch", None).await;
            assert_eq!(launch.data()["update"], Value::Null);
        }
    })
    .await;
}

#[tokio::test]
async fn failed_check_keeps_the_last_result() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let not_json = releases("this is no JSON").await;
        let no_version = releases(r#"{"tag_name": "latest"}"#).await;
        let not_found =
            StandIn::start(Router::new().route("/elsewhere", get(|| async { "" }))).await;
        for address in [
            latest_of(&not_json),
            latest_of(&no_version),
            latest_of(&not_found),
            // Nothing listens on the port of the discard service.
            "http://127.0.0.1:9/repos/owner/project/releases/latest".to_owned(),
        ] {
            let app = app_with_releases(database, &address);
            let answer = call(&app, As::Admin, "GET", "/update/check", None).await;
            assert_eq!(
                answer.status,
                StatusCode::INTERNAL_SERVER_ERROR,
                "{address}"
            );
            assert_eq!(
                answer.message(),
                json!("The newest version could not be read from GitHub")
            );
            assert_eq!(answer.data(), unchecked(), "{address}");
            assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        }

        // A failure after a check that worked keeps what that check found.
        let good = releases(r#"{"tag_name": "v999.0.0"}"#).await;
        let state =
            state_with(database, installed_settings(&[])).with_releases_api(&latest_of(&good));
        let app = organizr_api::router(state.clone()).expect("the route table is valid");
        let answer = call(&app, As::Admin, "GET", "/update/check", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        let app = organizr_api::router(state.with_releases_api(&latest_of(&not_json)))
            .expect("the route table is valid");
        let answer = call(&app, As::Admin, "GET", "/update/check", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.data()["latest"], json!("999.0.0"));
    })
    .await;
}

#[tokio::test]
async fn reset_of_the_most_access_gives_every_tab_to_the_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_with_releases(database, "http://127.0.0.1:9/");
        let tab = json!({
            "name": "Sonarr",
            "url": "https://sonarr.example.com/",
            "image": "plugins/images/tabs/sonarr.png",
            "group_id": 4,
            "group_id_max": 1,
        });
        let added = call(&app, As::Admin, "POST", "/tabs", Some(tab)).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        let most_access = async || -> Vec<Value> {
            let tabs = call(&app, As::Admin, "GET", "/tabs", None).await;
            tabs.data()["tabs"]
                .as_array()
                .expect("the tabs are a list")
                .iter()
                .map(|tab| tab["group_id_max"].clone())
                .collect()
        };
        assert!(most_access().await.contains(&json!(1)));

        for feature in ["GroupMax", "group-id-max", "group_id"] {
            let answer = call(
                &app,
                As::Admin,
                "GET",
                &format!("/update/reset/{feature}"),
                None,
            )
            .await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(
                answer.message(),
                json!(format!(
                    "Ran reset update feature for {}",
                    feature.to_lowercase()
                ))
            );
            assert_eq!(answer.data(), Value::Null);
            assert!(most_access().await.iter().all(|level| *level == json!(0)));
        }

        let unknown = call(&app, As::Admin, "GET", "/update/reset/tabs", None).await;
        assert_eq!(unknown.status, StatusCode::NOT_FOUND);
        assert_eq!(
            unknown.message(),
            json!("Feature not found in reset update")
        );
    })
    .await;
}

#[tokio::test]
async fn reset_and_check_refuse_a_request_from_another_site() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let stand_in = releases(r#"{"tag_name": "v999.0.0"}"#).await;
        let app = app_with_releases(database, &latest_of(&stand_in));
        let admin = seed::logged_in(database, &app, "ada", 0).await;
        for path in ["/api/v2/update/reset/groupmax", "/api/v2/update/check"] {
            let elsewhere = send(
                &app,
                request(
                    "GET",
                    path,
                    &[("cookie", &admin), ("sec-fetch-site", "cross-site")],
                    "",
                ),
            )
            .await;
            assert_eq!(elsewhere.status, StatusCode::UNAUTHORIZED, "{path}");
            let here = send(
                &app,
                request(
                    "GET",
                    path,
                    &[("cookie", &admin), ("sec-fetch-site", "same-origin")],
                    "",
                ),
            )
            .await;
            assert_eq!(here.status, StatusCode::OK, "{path}: {}", here.body);
        }
        assert_eq!(stand_in.seen().len(), 1);
    })
    .await;
}
