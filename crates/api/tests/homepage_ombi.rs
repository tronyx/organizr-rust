//! The Ombi item of the homepage, against a stand-in for Ombi.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, OMBI_KEY, OMBI_TOKEN};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const LIST: &str = "/homepage/ombi/requests";

fn settings(service: &StandIn, more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("homepageOmbiEnabled", json!(true)),
        ("homepageOmbiAuth", json!("4")),
        ("homepageOmbiRequestAuth", json!("4")),
        ("ombiURL", json!(service.url())),
        ("ombiToken", json!(OMBI_KEY)),
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
async fn requests_come_in_the_form_of_the_php_application_the_newest_first() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(database, &settings(&service, &[]));

        let all = call(&app, As::Admin, "GET", LIST, None).await;

        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(
            all.data()["count"],
            json!({"movie": 2, "tv": 1, "limit": 50, "offset": 0})
        );
        assert_eq!(
            all.data()["content"],
            json!([
                {
                    "id": 4711,
                    "title": "The Show",
                    "overview": "A show.",
                    "poster": "https://art.example.net/show.jpg",
                    "background": "",
                    "approved": false,
                    "available": false,
                    "denied": false,
                    "deniedReason": null,
                    "user": "pat",
                    "userAlias": "pat the alias",
                    "request_id": 32,
                    "request_date": "2020-01-05T10:00:00",
                    "release_date": "2018-09-01T00:00:00",
                    "type": "tv",
                    "icon": "mdi mdi-television",
                    "color": "grayish-blue-bg",
                },
                {
                    "id": 999,
                    "title": "Denied Film",
                    "overview": null,
                    "poster": "plugins/images/homepage/no-list.png",
                    "background": "",
                    "approved": false,
                    "available": false,
                    "denied": true,
                    "deniedReason": "No",
                    "user": "pat",
                    "userAlias": "pat the alias",
                    "request_id": 33,
                    "request_date": "2020-01-04T10:00:00",
                    "release_date": "2019-06-01T00:00:00",
                    "type": "movie",
                    "icon": "mdi mdi-filmstrip",
                    "color": "palette-Deep-Purple-900 bg white",
                },
                {
                    "id": 815,
                    "title": "The Film",
                    "overview": "A film.",
                    "poster": "https://image.tmdb.org/t/p/w300//poster815.jpg",
                    "background": "https://image.tmdb.org/t/p/w1280//back815.jpg",
                    "approved": true,
                    "available": true,
                    "denied": false,
                    "deniedReason": null,
                    "user": "robin",
                    "userAlias": "robin the alias",
                    "request_id": 31,
                    "request_date": "2020-01-03T10:00:00",
                    "release_date": "2019-05-01T00:00:00",
                    "type": "movie",
                    "icon": "mdi mdi-filmstrip",
                    "color": "palette-Deep-Purple-900 bg white",
                },
            ])
        );
        // Ombi takes the key in the header `ApiKey`.
        let seen = service.seen();
        assert_eq!(seen[0].path, "/api/v1/Request/movie");
        assert_eq!(seen[0].header("apikey"), Some(OMBI_KEY));
        assert_eq!(seen[1].path, "/api/v1/Request/tv");
    })
    .await;
}

#[tokio::test]
async fn type_limit_and_offset_of_the_path_choose_the_requests() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(database, &settings(&service, &[("ombiLimit", json!("2"))]));
        let get = async |path: &str| call(&app, As::Admin, "GET", path, None).await;

        // The limit of the settings holds when the path names none.
        let limited = get(LIST).await;
        assert_eq!(titles(&limited), ["The Show", "Denied Film"]);
        assert_eq!(limited.data()["count"]["limit"], 2);
        assert_eq!(limited.data()["count"]["movie"], 2);

        let films = get(&format!("{LIST}/movie")).await;
        assert_eq!(titles(&films), ["Denied Film", "The Film"]);
        assert_eq!(films.data()["count"]["tv"], 0);
        let asked = service.seen().len();
        let series = get(&format!("{LIST}/tv")).await;
        assert_eq!(titles(&series), ["The Show"]);
        assert_eq!(series.data()["count"]["movie"], 0);
        // Only the series were asked for.
        assert_eq!(service.seen().len(), asked + 1);

        let paged = get(&format!("{LIST}/both/1/1")).await;
        assert_eq!(titles(&paged), ["Denied Film"]);
        assert_eq!(
            paged.data()["count"],
            json!({"movie": 2, "tv": 1, "limit": 1, "offset": 1})
        );
        let all = get(&format!("{LIST}/both/10")).await;
        assert_eq!(titles(&all).len(), 3);
        // A limit that is no number is 0, as in the PHP application.
        let none = get(&format!("{LIST}/both/many")).await;
        assert_eq!(titles(&none), Vec::<String>::new());
        assert_eq!(none.data()["count"]["limit"], 0);
    })
    .await;
}

#[tokio::test]
async fn list_is_refused_when_the_item_is_off_or_not_for_the_caller_or_not_set_up() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;

        let app = installed_app_with(database, &settings(&service, &[]));
        let guest = call(&app, As::Guest, "GET", LIST, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            guest.message(),
            json!("User not approved to view this homepage item")
        );

        let off = installed_app_with(database, &[]);
        let refused = call(&off, As::Admin, "GET", LIST, None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("homepageOmbiEnabled module is not enabled")
        );

        let no_key = installed_app_with(database, &settings(&service, &[("ombiToken", json!(""))]));
        let refused = call(&no_key, As::Admin, "GET", LIST, None).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("ombiToken was not supplied"));
        let no_address =
            installed_app_with(database, &settings(&service, &[("ombiURL", json!(""))]));
        let refused = call(&no_address, As::Admin, "GET", LIST, None).await;
        assert_eq!(refused.message(), json!("ombiURL was not supplied"));
        assert_eq!(service.seen().len(), 0);

        // A wrong key: Ombi refuses, and the list is empty, as in the PHP application.
        let wrong = installed_app_with(
            database,
            &settings(&service, &[("ombiToken", json!("a-wrong-key"))]),
        );
        let empty = call(&wrong, As::Admin, "GET", LIST, None).await;
        assert_eq!(empty.status, StatusCode::OK, "{}", empty.body);
        assert_eq!(
            empty.data(),
            json!({"count": {"movie": 0, "tv": 0, "limit": 50, "offset": 0}, "content": []})
        );

        // An Ombi that cannot be reached: the error names no address.
        let down = installed_app_with(
            database,
            &settings(&service, &[("ombiURL", json!("http://127.0.0.1:9"))]),
        );
        let failed = call(&down, As::Admin, "GET", LIST, None).await;
        assert_eq!(failed.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!failed.body.contains("127.0.0.1"), "{}", failed.body);
        assert!(!failed.body.contains(OMBI_KEY), "{}", failed.body);
    })
    .await;
}

#[tokio::test]
async fn limited_list_shows_a_user_the_own_requests_and_the_ones_of_the_fallback_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let limited = settings(&service, &[("ombiLimitUser", json!(true))]);
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
        assert_eq!(none.data()["count"]["tv"], 0);
        // An Admin sees all requests.
        assert_eq!(
            titles(&call(&app, As::Admin, "GET", LIST, None).await),
            ["The Show", "Denied Film", "The Film"]
        );

        let mut with_fallback = limited.clone();
        with_fallback.push(("ombiFallbackUser", json!("PAT")));
        let app = installed_app_with(database, &with_fallback);
        assert_eq!(
            titles(&call(&app, As::User(&zed), "GET", LIST, None).await),
            ["The Show", "Denied Film"]
        );
    })
    .await;
}

#[tokio::test]
async fn only_admins_change_a_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(database, &settings(&service, &[]));
        let co_admin = seed::logged_in(database, &app, "cory", 1).await;
        let last = || {
            service
                .seen()
                .last()
                .map(|seen| (format!("{} {}", seen.method, seen.path), seen.json()))
        };

        for (method, action, asked, body, message) in [
            (
                "POST",
                "/approve",
                "POST /api/v1/Request/movie/approve",
                json!({"id": 31}),
                "Ombi Request has been approved",
            ),
            (
                "PUT",
                "/deny",
                "PUT /api/v1/Request/movie/deny",
                json!({"id": 31}),
                "Ombi Request has been denied",
            ),
            (
                "POST",
                "/available",
                "POST /api/v1/Request/movie/available",
                json!({"id": 31}),
                "Ombi Request has been marked available",
            ),
            (
                "POST",
                "/unavailable",
                "POST /api/v1/Request/movie/unavailable",
                json!({"id": 31}),
                "Ombi Request has been marked unavailable",
            ),
            (
                "DELETE",
                "",
                "DELETE /api/v1/Request/movie/31",
                Value::Null,
                "Ombi Request has been deleted",
            ),
        ] {
            let path = format!("{LIST}/movie/31{action}");
            let done = call(&app, As::Admin, method, &path, None).await;
            assert_eq!(done.status, StatusCode::OK, "{path}: {}", done.body);
            assert_eq!(done.message(), json!(message), "{path}");
            assert_eq!(last(), Some((asked.to_owned(), body)), "{path}");
        }
        assert_eq!(
            service
                .seen()
                .last()
                .and_then(|seen| seen.header("apikey").map(str::to_owned)),
            Some(OMBI_KEY.to_owned())
        );

        // A Co-Admin can too, and "season" is a series.
        let path = format!("{LIST}/season/32/approve");
        let done = call(&app, As::User(&co_admin), "POST", &path, None).await;
        assert_eq!(done.status, StatusCode::OK, "{}", done.body);
        assert_eq!(
            last().map(|(asked, _)| asked).as_deref(),
            Some("POST /api/v1/Request/tv/approve")
        );
        let done = call(&app, As::Admin, "DELETE", &format!("{LIST}/tv/32"), None).await;
        assert_eq!(done.status, StatusCode::OK, "{}", done.body);

        // Ombi refuses a request that it does not know.
        let unknown = call(
            &app,
            As::Admin,
            "POST",
            &format!("{LIST}/movie/77/approve"),
            None,
        )
        .await;
        assert_eq!(unknown.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(unknown.message(), json!("Ombi Error Occurred"));
    })
    .await;
}

#[tokio::test]
async fn user_and_guest_do_not_change_a_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(database, &settings(&service, &[]));
        let user = seed::logged_in(database, &app, "zed", 4).await;

        // A user sees the requests and cannot change them. An id that is no number
        // could name another path of Ombi.
        let asked = service.seen().len();
        for (method, action) in [
            ("POST", "/approve"),
            ("POST", "/available"),
            ("POST", "/unavailable"),
            ("PUT", "/deny"),
            ("DELETE", ""),
        ] {
            let path = format!("{LIST}/movie/31{action}");
            let refused = call(&app, As::User(&user), method, &path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
            let refused = call(&app, As::Guest, method, &path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED, "{path}");
        }
        let refused = call(
            &app,
            As::Admin,
            "DELETE",
            &format!("{LIST}/movie/31%2F..%2F32"),
            None,
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(refused.message(), json!("Id was not supplied"));
        assert_eq!(service.seen().len(), asked);
    })
    .await;
}

/// A request of a logged-in user whose browser also holds the token of Ombi.
async fn as_user_of_ombi(app: &Router, session: &str, path: &str) -> Answer {
    let cookies = format!("{session}; Auth={OMBI_TOKEN}");
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
async fn new_request_is_made_as_the_user() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(
            database,
            &settings(&service, &[("ombiTvDefault", json!("last"))]),
        );
        let user = seed::logged_in(database, &app, "robin", 4).await;
        let posted = |path: &str| {
            service
                .seen()
                .into_iter()
                .rfind(|seen| seen.method == "POST" && seen.path == path)
        };

        let film = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/123")).await;
        assert_eq!(film.status, StatusCode::OK, "{}", film.body);
        assert_eq!(film.message(), json!("Ombi Request submitted"));
        let sent = posted("/api/v1/Request/movie").expect("the request was posted");
        assert_eq!(sent.json(), json!({"theMovieDbId": 123}));
        // The token of the user is sent, and the key of the application is not.
        assert_eq!(
            sent.header("authorization"),
            Some(format!("Bearer {OMBI_TOKEN}").as_str())
        );
        assert_eq!(sent.header("apikey"), None);

        // A series: Ombi names its id in the TVDB, and the settings name the seasons.
        let series = as_user_of_ombi(&app, &user, &format!("{LIST}/tv/4712")).await;
        assert_eq!(series.status, StatusCode::OK, "{}", series.body);
        let sent = posted("/api/v1/Request/tv").expect("the request was posted");
        assert_eq!(
            sent.json(),
            json!({"tvDbId": 1234, "requestAll": false, "latestSeason": true, "firstSeason": false})
        );
        let unknown = as_user_of_ombi(&app, &user, &format!("{LIST}/tv/4713")).await;
        assert_eq!(unknown.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(unknown.message(), json!("Could not get TVDB Id"));

        // What is there or requested already is refused.
        let known = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/815")).await;
        assert_eq!(known.status, StatusCode::CONFLICT);
        assert_eq!(known.message(), json!("Request is already available"));
        let known = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/816")).await;
        assert_eq!(known.status, StatusCode::CONFLICT);
        assert_eq!(known.message(), json!("Request is already requested"));

        // Ombi refuses, or it does not know the film.
        let refused = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/500")).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(refused.message(), json!("Ombi Error Occurred"));
        let refused = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/404")).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);

        // A person without a token of Ombi, and an id that is no number.
        let asked = service.seen().len();
        let no_token = call(
            &app,
            As::User(&user),
            "POST",
            &format!("{LIST}/movie/123"),
            None,
        )
        .await;
        assert_eq!(no_token.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(no_token.message(), json!("User does not have Auth Cookie"));
        let no_number = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/abc")).await;
        assert_eq!(no_number.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(service.seen().len(), asked);
    })
    .await;
}

#[tokio::test]
async fn new_request_needs_the_level_for_requests() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(
            database,
            &settings(&service, &[("homepageOmbiRequestAuth", json!("1"))]),
        );
        let user = seed::logged_in(database, &app, "robin", 4).await;

        // The user sees the requests, and has no level to make one.
        let seen = call(&app, As::User(&user), "GET", LIST, None).await;
        assert_eq!(seen.status, StatusCode::OK);
        let refused = as_user_of_ombi(&app, &user, &format!("{LIST}/movie/123")).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert!(service.seen().iter().all(|seen| seen.method == "GET"));
    })
    .await;
}

#[tokio::test]
async fn homepage_lists_the_item_with_the_settings_of_its_look() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::ombi_requests().await;
        let app = installed_app_with(
            database,
            &settings(
                &service,
                &[
                    ("ombiAlias", json!(true)),
                    ("ombiDefaultFilterDenied", json!(false)),
                ],
            ),
        );
        let items = async |who| call(&app, who, "GET", "/homepage/items", None).await.data();

        assert_eq!(
            items(As::Admin).await,
            json!([{
                "key": "homepageOrderombi",
                "kind": "ombi",
                "id": "",
                "options": {
                    "alias": true,
                    "filters": {
                        "available": true,
                        "unavailable": true,
                        "approved": true,
                        "unapproved": true,
                        "denied": false,
                    },
                },
            }])
        );
        assert_eq!(items(As::Guest).await, json!([]));
    })
    .await;
}
