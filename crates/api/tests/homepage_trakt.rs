//! Trakt in the calendar of the homepage, and the login at Trakt, against a stand-in
//! for Trakt.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::{AppState, router};
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{
    self, TRAKT_CLIENT, TRAKT_CODE, TRAKT_NEXT_REFRESH, TRAKT_TOKENS,
};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

const CALENDAR: &str = "/homepage/trakt/calendar";
const LOGIN: &str = "/api/v2/oauth/trakt";

/// The address of the pages of Trakt in these tests. No request goes there.
const SITE: &str = "https://trakt.example";

type Changed = Vec<(&'static str, Value)>;

fn settings() -> Changed {
    vec![
        ("homepageTraktEnabled", json!(true)),
        ("homepageTraktAuth", json!("4")),
        ("traktClientId", json!(TRAKT_CLIENT.0)),
        ("traktClientSecret", json!(TRAKT_CLIENT.1)),
        ("traktAccessToken", json!(TRAKT_TOKENS.0)),
        ("calendarStartTrakt", json!("7")),
        ("calendarEndTrakt", json!("14")),
    ]
}

/// The settings with another value for one of them.
fn with(mut settings: Changed, key: &'static str, value: Value) -> Changed {
    settings.retain(|(known, _)| *known != key);
    settings.push((key, value));
    settings
}

fn state(database: &Database, api: &str, changed: &[(&'static str, Value)]) -> AppState {
    state_with(database, installed_settings(changed)).with_trakt(api, SITE)
}

fn app(database: &Database, api: &str, changed: &[(&'static str, Value)]) -> Router {
    router(state(database, api, changed)).expect("the route table is valid")
}

/// A request of the Admin to the route of the login, with a query and a cookie.
async fn login(app: &Router, query: &str, cookie: &str) -> Answer {
    let mut headers = vec![("token", seed::API_KEY)];
    if !cookie.is_empty() {
        headers.push(("cookie", cookie));
    }
    send(
        app,
        request("GET", &format!("{LOGIN}{query}"), &headers, ""),
    )
    .await
}

#[tokio::test]
async fn calendar_has_the_episodes_and_the_films_of_the_account() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;
        let app = app(database, &trakt.url(), &settings());

        let answer = call(&app, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let events = answer.data();
        let ids: Vec<&str> = events
            .as_array()
            .expect("a list")
            .iter()
            .filter_map(|event| event["id"].as_str())
            .collect();
        assert_eq!(ids, ["Trakt-Tv-1", "Trakt-Tv-2", "Trakt-Movie-1"]);
        // The first episode of a season that was not aired yet.
        assert_eq!(events[0]["title"], "The Show");
        assert_eq!(events[0]["start"], "2099-01-05T02:00:00.000Z");
        assert_eq!(events[0]["downloadFilter"], "text-primary animated flash");
        assert_eq!(events[0]["details"]["bottomTitle"], "S03E01 - The Return");
        // An episode that was aired.
        assert_eq!(events[1]["start"], "2001-03-04T20:00:01Z");
        assert_eq!(events[1]["downloadFilter"], "text-danger");
        assert_eq!(events[2]["start"], "2099-01-07");
        assert_eq!(events[2]["imagetypeFilter"], "film");
        assert_eq!(events[2]["details"]["bottomTitle"], "One more time");

        // The requests name the days of the settings, and carry the token in a header.
        let seen = trakt.seen();
        assert_eq!(seen.len(), 2);
        assert!(
            seen[0].path.starts_with("/calendars/my/shows/"),
            "{}",
            seen[0].path
        );
        assert!(
            seen[0].path.ends_with("/21?extended=full"),
            "{}",
            seen[0].path
        );
        assert!(
            seen[1].path.starts_with("/calendars/my/movies/"),
            "{}",
            seen[1].path
        );
        assert_eq!(
            seen[0].header("authorization"),
            Some(format!("Bearer {}", TRAKT_TOKENS.0).as_str())
        );
        assert_eq!(seen[0].header("trakt-api-version"), Some("2"));
        assert!(!seen[0].path.contains(TRAKT_TOKENS.0));

        // The calendar of the homepage has the events of Trakt too, and the item shows.
        let all = call(&app, As::Admin, "GET", "/homepage/calendar", None).await;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(all.data()["events"].as_array().map(Vec::len), Some(3));
        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(items.data()[0]["kind"], "calendar");
    })
    .await;
}

#[tokio::test]
async fn calendar_is_refused_as_every_homepage_item() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;
        let url = trakt.url();

        let off = app(
            database,
            &url,
            &with(settings(), "homepageTraktEnabled", json!(false)),
        );
        let answer = call(&off, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepageTraktEnabled module is not enabled")
        );

        let on = app(database, &url, &settings());
        let answer = call(&on, As::Guest, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            answer.message(),
            json!("User not approved to view this homepage item")
        );
        let user = seed::logged_in(database, &on, "uma", 4).await;
        let answer = call(&on, As::User(&user), "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        // A guest gets no event of Trakt in the calendar of the homepage.
        let all = call(&on, As::Guest, "GET", "/homepage/calendar", None).await;
        assert_eq!(all.data()["events"], json!([]));

        let no_login = app(
            database,
            &url,
            &with(settings(), "traktAccessToken", json!("")),
        );
        let answer = call(&no_login, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("traktAccessToken was not supplied"));
        assert_eq!(trakt.seen().len(), 2);
    })
    .await;
}

#[tokio::test]
async fn calendar_of_trakt_that_fails_names_no_token() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;

        // Trakt refuses the token: the calendar is empty.
        let wrong = with(settings(), "traktAccessToken", json!("a-wrong-token"));
        let refused = app(database, &trakt.url(), &wrong);
        let answer = call(&refused, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), json!([]));

        // Nothing listens at this address.
        let away = app(database, "http://127.0.0.1:1", &settings());
        let answer = call(&away, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("An error Occurred"));
        assert!(!answer.body.contains(TRAKT_TOKENS.0), "{}", answer.body);
        // The calendar of the homepage leaves Trakt out, and still answers.
        let all = call(&away, As::Admin, "GET", "/homepage/calendar", None).await;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(all.data()["events"], json!([]));
    })
    .await;
}

#[tokio::test]
async fn token_that_is_over_is_renewed_and_stored_as_a_secret() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;
        let mut old = with(settings(), "traktAccessToken", json!("an-old-token"));
        old.push(("traktRefreshToken", json!(TRAKT_TOKENS.1)));
        old.push(("traktAccessTokenExpires", json!("2020-01-01T00:00:00Z")));
        let state = state(database, &trakt.url(), &old);
        let app = router(state.clone()).expect("the route table is valid");

        let answer = call(&app, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data().as_array().map(Vec::len), Some(3));

        let seen = trakt.seen();
        assert_eq!(seen[0].path, "/oauth/token");
        assert_eq!(
            seen[0].json(),
            json!({
                "client_id": TRAKT_CLIENT.0,
                "client_secret": TRAKT_CLIENT.1,
                "redirect_uri": "http://organizr.example.com/api/v2/oauth/trakt",
                "refresh_token": TRAKT_TOKENS.1,
                "grant_type": "refresh_token",
            })
        );
        let now = state.settings();
        assert_eq!(now.trakt_access_token, TRAKT_TOKENS.0);
        assert_eq!(now.trakt_refresh_token, TRAKT_NEXT_REFRESH);
        assert_eq!(now.trakt_access_token_expires, "2100-01-02T00:00:00Z");

        // The database holds the tokens sealed.
        let stored = database
            .settings()
            .all()
            .await
            .expect("the settings can be read");
        let token = stored
            .iter()
            .find(|setting| setting.key == "traktAccessToken")
            .expect("the token is stored");
        assert!(!token.value.contains(TRAKT_TOKENS.0), "{}", token.value);
        let refresh = stored
            .iter()
            .find(|setting| setting.key == "traktRefreshToken")
            .expect("the refresh token is stored");
        assert!(
            !refresh.value.contains(TRAKT_NEXT_REFRESH),
            "{}",
            refresh.value
        );

        // A token that is not over is not renewed.
        let before = trakt.seen().len();
        let answer = call(&app, As::Admin, "GET", CALENDAR, None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(trakt.seen().len(), before + 2);
    })
    .await;
}

#[tokio::test]
async fn login_sends_the_admin_to_trakt_and_stores_the_tokens_of_the_code() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;
        let fresh: Changed = vec![
            ("traktClientId", json!(TRAKT_CLIENT.0)),
            ("traktClientSecret", json!(TRAKT_CLIENT.1)),
        ];
        let state = state(database, &trakt.url(), &fresh);
        let app = router(state.clone()).expect("the route table is valid");

        // Without a code, the browser goes to Trakt, with a state that a cookie keeps.
        let start = login(&app, "", "").await;
        assert_eq!(start.status, StatusCode::FOUND, "{}", start.body);
        let location = start.headers["location"].to_str().expect("text").to_owned();
        let expected = format!(
            "{SITE}/oauth/authorize?response_type=code&client_id={}&redirect_uri=http%3A%2F%2Forganizr.example.com%2Fapi%2Fv2%2Foauth%2Ftrakt&state=",
            TRAKT_CLIENT.0
        );
        assert!(location.starts_with(&expected), "{location}");
        let login_state = location.rsplit('=').next().expect("a state").to_owned();
        assert_eq!(login_state.len(), 32);
        let cookie = start.cookie("organizr_trakt_state").expect("the cookie of the login");
        assert_eq!(cookie, format!("organizr_trakt_state={login_state}"));
        assert!(start.cookies[0].contains("HttpOnly"), "{}", start.cookies[0]);
        assert!(start.cookies[0].contains("Path=/api/v2/oauth/trakt"));
        assert!(!location.contains(TRAKT_CLIENT.1));

        // An answer with another state, or without the cookie, stores nothing.
        let query = format!("?code={TRAKT_CODE}&state={login_state}");
        for (query, cookie) in [
            (format!("?code={TRAKT_CODE}&state=another"), cookie.as_str()),
            (format!("?code={TRAKT_CODE}"), cookie.as_str()),
            (query.clone(), ""),
        ] {
            let answer = login(&app, &query, cookie).await;
            assert_eq!(answer.status, StatusCode::OK);
            assert_eq!(answer.body, "Invalid state");
        }
        assert_eq!(trakt.seen().len(), 0);
        assert_eq!(state.settings().trakt_access_token, "");

        // A code that Trakt refuses.
        let wrong = login(&app, &format!("?code=wrong&state={login_state}"), &cookie).await;
        assert_eq!(wrong.status, StatusCode::OK);
        assert!(wrong.body.contains("Trakt refused the request for tokens"), "{}", wrong.body);
        assert!(!wrong.body.contains(TRAKT_CLIENT.1), "{}", wrong.body);
        assert_eq!(state.settings().trakt_access_token, "");

        // The code of Trakt buys the tokens.
        let done = login(&app, &query, &cookie).await;
        assert_eq!(done.status, StatusCode::OK, "{}", done.body);
        assert!(done.body.contains("Details Saved"), "{}", done.body);
        assert!(done.body.contains("<title>Trakt OAuth</title>"));
        assert_eq!(
            done.headers["content-type"].to_str().expect("text"),
            "text/html;charset=UTF-8"
        );
        assert!(!done.body.contains(TRAKT_TOKENS.0));
        // The cookie of the login is taken away.
        assert!(done.cookies.iter().any(|cookie| cookie.starts_with("organizr_trakt_state=;")));
        let now = state.settings();
        assert_eq!(now.trakt_access_token, TRAKT_TOKENS.0);
        assert_eq!(now.trakt_refresh_token, TRAKT_TOKENS.1);
        assert_eq!(now.trakt_access_token_expires, "2100-01-02T00:00:00Z");
        let sent = trakt.seen().last().expect("a request").json();
        assert_eq!(sent["grant_type"], "authorization_code");
        assert_eq!(sent["code"], TRAKT_CODE);
        assert_eq!(sent["redirect_uri"], "http://organizr.example.com/api/v2/oauth/trakt");
    })
    .await;
}

#[tokio::test]
async fn login_is_for_an_admin_or_a_co_admin_with_an_api_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let trakt = homepage_services::trakt().await;
        let ready = app(database, &trakt.url(), &settings());

        let guest = call(&ready, As::Guest, "GET", "/oauth/trakt", None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        let user = seed::logged_in(database, &ready, "uma", 4).await;
        let answer = call(
            &ready,
            As::User(&user),
            "GET",
            "/oauth/trakt?code=x&state=y",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        let co_admin = seed::logged_in(database, &ready, "cora", 1).await;
        let answer = call(&ready, As::User(&co_admin), "GET", "/oauth/trakt", None).await;
        assert_eq!(answer.status, StatusCode::FOUND);

        for key in ["traktClientId", "traktClientSecret"] {
            let missing = app(database, &trakt.url(), &with(settings(), key, json!("")));
            let answer = call(&missing, As::Admin, "GET", "/oauth/trakt", None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(answer.message(), json!(format!("{key} was not supplied")));
        }
        assert_eq!(trakt.seen().len(), 0);
    })
    .await;
}
