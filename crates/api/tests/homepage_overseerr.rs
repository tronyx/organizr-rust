//! The Overseerr item of the homepage, against a stand-in for Overseerr.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, OVERSEERR_KEY, OVERSEERR_SESSION};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const LIST: &str = "/homepage/overseerr/requests";

fn settings(service: &StandIn, more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("homepageOverseerrEnabled", json!(true)),
        ("homepageOverseerrAuth", json!("4")),
        ("homepageOverseerrRequestAuth", json!("4")),
        ("overseerrURL", json!(service.url())),
        ("overseerrToken", json!(OVERSEERR_KEY)),
    ];
    settings.extend(more.iter().cloned());
    settings
}

fn titles(answer: &Answer) -> Vec<String> {
    answer.data()["content"]
        .as_array()
        .expect("a list of requests")
        .iter()
        .map(|request| request["title"].as_str().unwrap_or("?").to_owned())
        .collect()
}

#[tokio::test]
async fn requests_come_with_the_details_of_overseerr_the_newest_first() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let app = installed_app_with(database, &settings(&service, &[]));

        let all = call(&app, As::Admin, "GET", LIST, None).await;

        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        // The request for the film without details is left out. It still counts.
        assert_eq!(
            all.data()["count"],
            json!({"movie": 2, "tv": 1, "limit": 50, "offset": 0})
        );
        assert_eq!(
            all.data()["content"],
            json!([
                {
                    "id": 4711,
                    "approved": false,
                    "available": false,
                    "denied": false,
                    "deniedReason": "n/a",
                    "user": "pat",
                    "userAlias": "pat",
                    "request_id": 32,
                    "request_date": "2020-01-05T10:00:00.000Z",
                    "type": "tv",
                    "icon": "filmstrip",
                    "color": "grayish-blue-bg",
                    "title": "The Show",
                    "release_date": "2018-09-01",
                    "background": "",
                    "poster": "plugins/images/homepage/no-list.png",
                    "overview": "A show.",
                },
                {
                    "id": 815,
                    "approved": true,
                    "available": true,
                    "denied": false,
                    "deniedReason": "n/a",
                    "user": "robin",
                    "userAlias": "robin",
                    "request_id": 31,
                    "request_date": "2020-01-03T10:00:00.000Z",
                    "type": "movie",
                    "icon": "filmstrip",
                    "color": "palette-Deep-Purple-900 bg white",
                    "title": "The Film",
                    "release_date": "2019-05-01",
                    "background": "https://image.tmdb.org/t/p/w1280//back815.jpg",
                    "poster": "https://image.tmdb.org/t/p/w300//poster815.jpg",
                    "overview": "A film.",
                },
            ])
        );
        let seen = service.seen();
        assert_eq!(seen[0].path, "/api/v1/request?take=50&skip=0");
        assert_eq!(seen[0].header("x-api-key"), Some(OVERSEERR_KEY));

        let paged = call(&app, As::Admin, "GET", &format!("{LIST}/all/10/20"), None).await;
        assert_eq!(paged.data()["count"]["limit"], 10);
        assert_eq!(paged.data()["count"]["offset"], 20);
        assert!(
            service
                .seen()
                .iter()
                .any(|seen| seen.path == "/api/v1/request?take=10&skip=20")
        );

        let guest = call(&app, As::Guest, "GET", LIST, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn limited_list_shows_a_user_the_own_requests_and_the_ones_of_the_fallback_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let limited = settings(&service, &[("overseerrLimitUser", json!(true))]);
        let app = installed_app_with(database, &limited);
        let robin = seed::logged_in(database, &app, "Robin", 4).await;
        let zed = seed::logged_in(database, &app, "zed", 4).await;

        // The name is compared without regard to the case of the letters.
        assert_eq!(
            titles(&call(&app, As::User(&robin), "GET", LIST, None).await),
            ["The Film"]
        );
        let none = call(&app, As::User(&zed), "GET", LIST, None).await;
        assert_eq!(titles(&none), Vec::<String>::new());
        assert_eq!(none.data()["count"]["movie"], 0);
        // An Admin sees all requests.
        assert_eq!(
            titles(&call(&app, As::Admin, "GET", LIST, None).await),
            ["The Show", "The Film"]
        );

        let mut with_fallback = limited.clone();
        with_fallback.push(("overseerrFallbackUser", json!("PAT")));
        let app = installed_app_with(database, &with_fallback);
        assert_eq!(
            titles(&call(&app, As::User(&zed), "GET", LIST, None).await),
            ["The Show"]
        );
    })
    .await;
}

#[tokio::test]
async fn only_admins_change_a_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let app = installed_app_with(database, &settings(&service, &[]));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let last = || {
            service
                .seen()
                .last()
                .map(|seen| format!("{} {}", seen.method, seen.path))
        };

        for (method, action, asked, message) in [
            (
                "POST",
                "/approve",
                "POST /api/v1/request/31/approve",
                "Overseerr Request has been approved",
            ),
            (
                "POST",
                "/pending",
                "POST /api/v1/request/31/pending",
                "Overseerr Request has been approved",
            ),
            (
                "PUT",
                "/deny",
                "POST /api/v1/request/31/decline",
                "Overseerr Request has been denied",
            ),
            (
                "POST",
                "/available",
                "POST /api/v1/media/71/available",
                "Overseerr Request has been marked available",
            ),
            (
                "POST",
                "/unavailable",
                "POST /api/v1/media/71/pending",
                "Overseerr Request has been marked unavailable",
            ),
            (
                "DELETE",
                "",
                "DELETE /api/v1/request/31",
                "Overseerr Request has been deleted",
            ),
        ] {
            let path = format!("{LIST}/movie/31{action}");
            let done = call(&app, As::Admin, method, &path, None).await;
            assert_eq!(done.status, StatusCode::OK, "{path}: {}", done.body);
            assert_eq!(done.message(), json!(message), "{path}");
            assert_eq!(last().as_deref(), Some(asked), "{path}");
        }

        // Overseerr says why it refuses.
        let unknown = call(
            &app,
            As::Admin,
            "POST",
            &format!("{LIST}/movie/77/approve"),
            None,
        )
        .await;
        assert_eq!(unknown.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(unknown.message(), json!("Request not found."));

        // A user sees the requests and cannot change them. An id that is no number
        // could name another path of Overseerr.
        let asked = service.seen().len();
        let refused = call(
            &app,
            As::User(&user),
            "POST",
            &format!("{LIST}/movie/31/approve"),
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let refused = call(
            &app,
            As::Admin,
            "DELETE",
            &format!("{LIST}/movie/31%2F..%2F32"),
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(service.seen().len(), asked);
    })
    .await;
}

/// A request of a logged-in user whose browser also holds the session of Overseerr.
async fn as_user_of_overseerr(app: &Router, session: &str, path: &str) -> Answer {
    let cookies = format!("{session}; connect.sid={OVERSEERR_SESSION}");
    let headers = [
        ("cookie", cookies.as_str()),
        ("origin", "http://organizr.example.com"),
        ("host", "organizr.example.com"),
    ];
    send(
        app,
        request("POST", &format!("/api/v2{path}"), &headers, ""),
    )
    .await
}

#[tokio::test]
async fn new_request_is_made_as_the_user_with_the_default_service() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let app = installed_app_with(
            database,
            &settings(&service, &[("overseerrPrefer4K", json!(true))]),
        );
        let user = seed::logged_in(database, &app, "robin", 4).await;
        let posted = || {
            service
                .seen()
                .into_iter()
                .rfind(|seen| seen.method == "POST" && seen.path == "/api/v1/request")
        };

        // A film: the 4K Radarr is the default that the settings prefer.
        let film = as_user_of_overseerr(&app, &user, &format!("{LIST}/movie/123")).await;
        assert_eq!(film.status, StatusCode::OK, "{}", film.body);
        assert_eq!(film.message(), json!("Overseerr Request submitted"));
        let sent = posted().expect("the request was posted");
        assert_eq!(
            sent.json(),
            json!({"mediaId": 123, "mediaType": "movie", "is4k": true, "serverId": 1, "profileId": 9, "tags": []})
        );
        assert_eq!(
            sent.header("cookie"),
            Some(format!("connect.sid={OVERSEERR_SESSION}").as_str())
        );
        assert_eq!(sent.header("x-api-key"), None);

        // A series with the default "all": every season but the specials.
        let series = as_user_of_overseerr(&app, &user, &format!("{LIST}/tv/4712")).await;
        assert_eq!(series.status, StatusCode::INTERNAL_SERVER_ERROR, "{}", series.body);
        assert_eq!(series.message(), json!("Error getting series information"));

        // What is requested already is refused, and so is a person without a session
        // of Overseerr.
        let known = as_user_of_overseerr(&app, &user, &format!("{LIST}/movie/815")).await;
        assert_eq!(known.status, StatusCode::CONFLICT);
        assert_eq!(known.message(), json!("Request is already available"));
        let known = as_user_of_overseerr(&app, &user, &format!("{LIST}/tv/4711")).await;
        assert_eq!(known.message(), json!("Request is already requested"));
        let no_session = call(&app, As::User(&user), "POST", &format!("{LIST}/movie/123"), None).await;
        assert_eq!(no_session.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(no_session.message(), json!("User does not have Auth Cookie"));

        // Overseerr says why it refuses a request.
        let refused = as_user_of_overseerr(&app, &user, &format!("{LIST}/movie/500")).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(refused.message(), json!("No seasons available to request"));
    })
    .await;
}

#[tokio::test]
async fn details_name_no_requester_for_a_user_whose_requests_are_limited() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let path = "/homepage/overseerr/metadata/movie/815";
        for (limited, level, sees) in [(true, 4, false), (true, 1, true), (false, 4, true)] {
            let app = installed_app_with(
                database,
                &settings(&service, &[("overseerrLimitUser", json!(limited))]),
            );
            let caller =
                seed::logged_in(database, &app, &format!("u{level}{limited}"), level).await;
            let details = call(&app, As::User(&caller), "GET", path, None).await;
            assert_eq!(details.status, StatusCode::OK, "{}", details.body);
            assert_eq!(details.data()["title"], "The Film");
            // With the limit, a user sees only their own requests, so the details name
            // nobody who asked for the film (D-151).
            let requests = &details.data()["mediaInfo"]["requests"];
            assert_eq!(
                requests.is_array(),
                sees,
                "{limited} {level}: {}",
                details.body
            );
            assert_eq!(details.data()["mediaInfo"]["status"], 5);
        }
    })
    .await;
}

#[tokio::test]
async fn details_come_as_overseerr_gives_them_for_callers_who_may_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::overseerr_requests().await;
        let app = installed_app_with(
            database,
            &settings(&service, &[("homepageOverseerrRequestAuth", json!("1"))]),
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let path = "/homepage/overseerr/metadata/tv/4711";

        let details = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(details.status, StatusCode::OK, "{}", details.body);
        assert_eq!(details.data()["name"], "The Show");
        assert_eq!(details.data()["externalIds"]["tvdbId"], 1234);

        // The user sees the requests, and has no level to make one.
        let refused = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let refused = call(
            &app,
            As::User(&user),
            "POST",
            &format!("{LIST}/movie/123"),
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

        let asked = service.seen().len();
        for wrong in [
            "/homepage/overseerr/metadata/user/1",
            "/homepage/overseerr/metadata/tv/abc",
        ] {
            let refused = call(&app, As::Admin, "GET", wrong, None).await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{wrong}");
        }
        assert_eq!(service.seen().len(), asked);
        let missing = call(
            &app,
            As::Admin,
            "GET",
            "/homepage/overseerr/metadata/movie/999",
            None,
        )
        .await;
        assert_eq!(missing.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(missing.message(), json!("Error getting series information"));
    })
    .await;
}
