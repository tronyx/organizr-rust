//! The API SOCKS, against a stand-in for an application.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{StatusCode, header};
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, SOCKS_ANSWER};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::login_servers::closed_port;
use organizr_testkit::services::{Seen, StandIn};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// The key of Sonarr in the settings. Organizr never sends it.
const STORED_KEY: &str = "stored-key-of-sonarr-0123";

/// The key of Sonarr that the caller brings.
const CALLERS_KEY: &str = "callers-key-of-sonarr-4567";

/// The settings of Sonarr with the API SOCKS on, for the given level and address.
fn sonarr(level: &str, url: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("sonarrSocksEnabled", json!(true)),
        ("sonarrSocksAuth", json!(level)),
        ("sonarrURL", json!(url)),
        ("sonarrToken", json!(STORED_KEY)),
    ]
}

/// Sends a request with the given headers to a path below `/api/v2`.
async fn with_headers(
    app: &Router,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> Answer {
    send(
        app,
        request(method, &format!("/api/v2{path}"), headers, body),
    )
    .await
}

/// The one request that the stand-in got.
fn only(service: &StandIn) -> Seen {
    let seen = service.seen();
    assert_eq!(seen.len(), 1, "{seen:?}");
    seen[0].clone()
}

fn assert_passed_on(answer: &Answer) {
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.body, SOCKS_ANSWER);
    assert_eq!(
        answer.headers[header::CONTENT_TYPE],
        "application/json;charset=UTF-8"
    );
}

async fn app_for(database: &Database, settings: &[(&str, Value)]) -> Router {
    seed::installed(database).await;
    installed_app_with(database, settings)
}

#[tokio::test]
async fn unsupported_application_gets_the_text_of_php() {
    on_each_engine(async |database| {
        let app = app_for(database, &[]).await;
        for path in [
            "/socks/plex/library",
            "/socks/Sonarr/api/v3/queue",
            "/multiple/socks/ombi/1/api",
        ] {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{path}");
            assert_eq!(answer.message(), "Application not supported for socks");
            assert_eq!(
                answer.headers[header::CONTENT_TYPE],
                "application/json;charset=UTF-8"
            );
        }
        // Without the slash after the application, no route takes the request.
        let answer = call(&app, As::Admin, "GET", "/socks/sonarr", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), "Endpoint Not Found or Defined");
    })
    .await;
}

#[tokio::test]
async fn switched_off_and_wrong_form_get_the_answers_of_php() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        // The answers of the recording of the PHP application (`routing.json`).
        let app = app_for(database, &[]).await;
        let answer = call(&app, As::Guest, "GET", "/socks/sonarr/api/v3/queue", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), "SOCKS module is not enabled");
        let answer = call(
            &app,
            As::Guest,
            "POST",
            "/multiple/socks/radarr/2/api/v3/command",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            "Multiple endpoint accessed but multiple URLs not found in field, please use \
             /api/v2/socks endpoint"
        );

        let mut settings = sonarr("999", &format!("{0},{0}", service.url()));
        let app = installed_app_with(database, &settings);
        let answer = call(&app, As::Admin, "GET", "/socks/sonarr/api", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            "Multiple URLs found in field, please use /api/v2/multiple/socks endpoint"
        );

        // A caller without the level of a switched-off module: the last check answers.
        settings[0] = ("sonarrSocksEnabled", json!(false));
        settings[1] = ("sonarrSocksAuth", json!("0"));
        settings[2] = ("sonarrURL", json!(service.url()));
        let app = installed_app_with(database, &settings);
        let answer = call(&app, As::Guest, "GET", "/socks/sonarr/api", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(answer.message(), "Not Authorized");
        let answer = call(&app, As::Admin, "GET", "/socks/sonarr/api", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), "SOCKS module is not enabled");
        assert_eq!(service.seen(), Vec::<Seen>::new());
    })
    .await;
}

#[tokio::test]
async fn caller_needs_the_level_of_the_setting() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(database, &sonarr("1", &service.url())).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let path = "/socks/sonarr/api/v3/queue";

        assert_passed_on(&call(&app, As::Admin, "GET", path, None).await);
        assert_passed_on(&call(&app, As::User(&co_admin), "GET", path, None).await);
        for who in [As::User(&user), As::Guest] {
            let answer = call(&app, who, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{who:?}");
            assert_eq!(answer.message(), "Not Authorized");
        }
        assert_eq!(service.seen().len(), 2);

        // The level of a guest lets everybody in.
        let open = installed_app_with(database, &sonarr("999", &service.url()));
        assert_passed_on(&call(&open, As::Guest, "GET", path, None).await);
        let user_level = installed_app_with(database, &sonarr("4", &service.url()));
        assert_passed_on(&call(&user_level, As::User(&user), "GET", path, None).await);
        let answer = call(&user_level, As::Guest, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn key_of_the_caller_goes_and_the_stored_key_never_does() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(database, &sonarr("999", &service.url())).await;

        let answer = with_headers(
            &app,
            "GET",
            "/socks/sonarr/api/v3/series",
            &[("x-api-key", CALLERS_KEY)],
            "",
        )
        .await;
        assert_passed_on(&answer);
        let seen = only(&service);
        assert_eq!(seen.path, "/api/v3/series");
        assert_eq!(seen.header("x-api-key"), Some(CALLERS_KEY));

        // Without a key of the caller, the request goes without a key.
        let answer = call(&app, As::Guest, "GET", "/socks/sonarr/api/v3/series", None).await;
        assert_passed_on(&answer);
        let seen = service.seen();
        assert_eq!(seen[1].header("x-api-key"), None);
        for seen in &seen {
            assert!(!format!("{seen:?}").contains(STORED_KEY), "{seen:?}");
        }
        assert!(!answer.body.contains(STORED_KEY));
    })
    .await;
}

#[tokio::test]
async fn cookies_authorization_and_other_headers_stay_in_organizr() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(database, &sonarr("999", &service.url())).await;
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let answer = with_headers(
            &app,
            "GET",
            &format!("/socks/sonarr/api/v3/queue?apikey={}&page=2", seed::API_KEY),
            &[
                ("cookie", &user),
                ("token", seed::API_KEY),
                ("authorization", "Basic dXNlcjpwYXNzd29yZA=="),
                ("proxy-authorization", "Basic dXNlcjpwYXNzd29yZA=="),
                ("x-forwarded-for", "203.0.113.9"),
                ("connection", "keep-alive, x-secret"),
                ("x-secret", "of the caller"),
                ("te", "trailers"),
                ("upgrade", "websocket"),
                ("x-api-key", CALLERS_KEY),
            ],
            "",
        )
        .await;
        assert_passed_on(&answer);
        let seen = only(&service);
        // The API key of Organizr stays out of the query; the other fields go.
        assert_eq!(seen.path, "/api/v3/queue?page=2");
        let mut names: Vec<&str> = seen.headers.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            ["accept", "host", "user-agent", "x-api-key"],
            "{seen:?}"
        );
        assert!(!format!("{seen:?}").contains(seed::API_KEY));
        assert!(!format!("{seen:?}").contains("organizr_token_"));
        assert_eq!(
            seen.header("host"),
            Some(service.url().trim_start_matches("http://"))
        );
    })
    .await;
}

#[tokio::test]
async fn path_that_leaves_the_address_is_refused() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(
            database,
            &sonarr("999", &format!("{}/sonarr", service.url())),
        )
        .await;
        for path in [
            "/socks/sonarr/../../etc/passwd",
            "/socks/sonarr/api/%2e%2e/%2e%2e/etc",
            "/socks/sonarr/api/%2E%2E/x",
            "/socks/sonarr/api/.%2e/x",
            "/socks/sonarr/api/%252e%252e/x",
            "/socks/sonarr/api%2f..%2f..%2fx",
            "/socks/sonarr/api/%5c..%5cx",
            "/socks/sonarr/api/%0d%0aHost:%20other.example",
            "/socks/sonarr/./x",
        ] {
            let answer = call(&app, As::Admin, "GET", path, None).await;
            assert_eq!(
                answer.status,
                StatusCode::BAD_REQUEST,
                "{path}: {}",
                answer.body
            );
            assert_eq!(
                answer.message(),
                "The path must not leave the address of the application"
            );
        }
        assert!(service.seen().is_empty(), "{:?}", service.seen());

        // A path that only looks odd stays below the address.
        let answer = call(
            &app,
            As::Admin,
            "GET",
            "/socks/sonarr//@other.example/x",
            None,
        )
        .await;
        assert_passed_on(&answer);
        assert_eq!(only(&service).path, "/sonarr/@other.example/x");
    })
    .await;
}

#[tokio::test]
async fn query_and_path_go_on_as_php_sends_them() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(
            database,
            &sonarr("999", &format!("{}/sonarr/", service.url())),
        )
        .await;

        for (path, expected) in [
            (
                "/socks/sonarr/api/v3/series?sortKey=title&page=2",
                "/sonarr/api/v3/series?sortKey=title&page=2",
            ),
            ("/socks/sonarr/api/v3/series/", "/sonarr/api/v3/series"),
            ("/socks/sonarr/api/v3/?q=a%20b", "/sonarr/api/v3/?q=a+b"),
            ("/socks/sonarr/", "/sonarr"),
            (
                "/socks/sonarr/api/v3/name%20one",
                "/sonarr/api/v3/name%20one",
            ),
        ] {
            assert_passed_on(&call(&app, As::Admin, "GET", path, None).await);
            let seen = service.seen();
            assert_eq!(
                seen.last().map(|seen| seen.path.as_str()),
                Some(expected),
                "{path}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn methods_and_bodies_go_as_php_sends_them() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(database, &sonarr("999", &service.url())).await;
        let path = "/socks/sonarr/api/v3/command";
        let json_type = [("content-type", "application/json")];
        let last = || service.seen().last().cloned().expect("a request went");

        let body = r#"{"name":"RefreshSeries"}"#;
        assert_passed_on(&with_headers(&app, "POST", path, &json_type, body).await);
        let seen = last();
        assert_eq!((seen.method.as_str(), seen.body.as_str()), ("POST", body));
        assert_eq!(seen.header("content-type"), Some("application/json"));

        assert_passed_on(&with_headers(&app, "PUT", path, &json_type, body).await);
        assert_eq!(
            (last().method.as_str(), last().body.as_str()),
            ("PUT", body)
        );

        let form = [("content-type", "application/x-www-form-urlencoded")];
        assert_passed_on(&with_headers(&app, "POST", path, &form, "name=a+b&x=1").await);
        assert_eq!(last().body, "name=a+b&x=1");

        // PHP sends no body of another type, and none with a DELETE.
        let text = [("content-type", "text/plain")];
        assert_passed_on(&with_headers(&app, "POST", path, &text, "plain").await);
        assert_eq!(last().body, "");
        assert_passed_on(
            &with_headers(
                &app,
                "DELETE",
                "/socks/sonarr/api/v3/series/1",
                &json_type,
                body,
            )
            .await,
        );
        assert_eq!(
            (last().method.as_str(), last().body.as_str()),
            ("DELETE", "")
        );
        assert_eq!(last().path, "/api/v3/series/1");

        // Every other method of Slim's `any` goes as GET, without a body.
        for method in ["PATCH", "OPTIONS"] {
            assert_passed_on(&with_headers(&app, method, path, &json_type, body).await);
            assert_eq!((last().method.as_str(), last().body.as_str()), ("GET", ""));
        }

        // A body over the limit is refused.
        let large = "x".repeat(2 * 1024 * 1024 + 1);
        let answer = with_headers(&app, "POST", path, &json_type, &large).await;
        assert_eq!(answer.status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(service.seen().len(), 7);

        // From another site, a request that changes something is refused (D-03).
        let answer = with_headers(
            &app,
            "POST",
            path,
            &[
                ("content-type", "application/json"),
                ("sec-fetch-site", "cross-site"),
            ],
            body,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), 7);
    })
    .await;
}

#[tokio::test]
async fn multiple_form_picks_the_install_by_its_place() {
    on_each_engine(async |database| {
        let first = homepage_services::socks_app().await;
        let second = homepage_services::socks_app().await;
        let settings = vec![
            ("radarrSocksEnabled", json!(true)),
            ("radarrSocksAuth", json!("999")),
            (
                "radarrURL",
                json!(format!("{}, {}/radarr", first.url(), second.url())),
            ),
        ];
        let app = app_for(database, &settings).await;

        let answer = call(
            &app,
            As::Guest,
            "GET",
            "/multiple/socks/radarr/2/api/v3/movie",
            None,
        )
        .await;
        assert_passed_on(&answer);
        assert_eq!(first.seen(), Vec::<Seen>::new());
        assert_eq!(only(&second).path, "/radarr/api/v3/movie");
        assert_passed_on(&call(&app, As::Guest, "GET", "/multiple/socks/radarr/1/api", None).await);
        assert_eq!(only(&first).path, "/api");

        for instance in ["3", "x", "-1", "01x"] {
            let answer = call(
                &app,
                As::Guest,
                "GET",
                &format!("/multiple/socks/radarr/{instance}/api"),
                None,
            )
            .await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{instance}");
            assert_eq!(answer.message(), "Instance not found in field");
        }
        // PHP takes the instance `0` for no instance.
        let answer = call(&app, As::Guest, "GET", "/multiple/socks/radarr/0/api", None).await;
        assert_eq!(
            answer.message(),
            "Multiple URLs found in field, please use /api/v2/multiple/socks endpoint"
        );
        assert_eq!(first.seen().len() + second.seen().len(), 2);
    })
    .await;
}

#[tokio::test]
async fn answer_is_json_with_status_200_whatever_the_application_answers() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let app = app_for(database, &sonarr("999", &service.url())).await;

        let answer = call(&app, As::Guest, "GET", "/socks/sonarr/teapot", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.body, "<p>short and stout</p>");
        assert_eq!(
            answer.headers[header::CONTENT_TYPE],
            "application/json;charset=UTF-8"
        );
        assert_eq!(answer.headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");

        // A redirect is not followed, and its header does not come back.
        let answer = call(&app, As::Guest, "GET", "/socks/sonarr/moved", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.body, "");
        assert!(answer.headers.get(header::LOCATION).is_none());
        assert_eq!(service.seen().len(), 2);
    })
    .await;
}

#[tokio::test]
async fn application_that_cannot_be_reached_gets_500_without_the_address() {
    on_each_engine(async |database| {
        let port = closed_port().await;
        let address = format!("http://127.0.0.1:{port}");
        let app = app_for(database, &sonarr("999", &address)).await;

        let answer = call(
            &app,
            As::Guest,
            "GET",
            &format!("/socks/sonarr/api?apikey={CALLERS_KEY}"),
            None,
        )
        .await;
        assert_eq!(
            answer.status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{}",
            answer.body
        );
        assert_eq!(answer.json()["response"]["result"], "error");
        assert!(!answer.body.contains(&port.to_string()), "{}", answer.body);
        assert!(!answer.body.contains(CALLERS_KEY), "{}", answer.body);
        assert!(!answer.body.contains(STORED_KEY), "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn address_that_is_only_a_path_gets_the_host_of_the_request() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let host = service.url().trim_start_matches("http://").to_owned();
        let app = app_for(database, &sonarr("999", "/sonarr/")).await;
        let mut request = request("GET", "/api/v2/socks/sonarr/api/v3/queue?page=2", &[], "");
        request.headers_mut().insert(
            header::HOST,
            host.parse().expect("the host is a header value"),
        );

        let answer = send(&app, request).await;
        assert_passed_on(&answer);
        let seen = only(&service);
        assert_eq!(seen.path, "/sonarr/api/v3/queue?page=2");
        assert_eq!(seen.header("host"), Some(host.as_str()));
    })
    .await;
}

#[tokio::test]
async fn keys_in_the_query_and_the_path_reach_the_other_applications() {
    on_each_engine(async |database| {
        let service = homepage_services::socks_app().await;
        let settings = vec![
            ("tautulliSocksEnabled", json!(true)),
            ("tautulliSocksAuth", json!("999")),
            ("tautulliURL", json!(service.url())),
            ("nzbgetSocksEnabled", json!(true)),
            ("nzbgetSocksAuth", json!("999")),
            ("nzbgetURL", json!(service.url())),
            ("sabnzbdSocksEnabled", json!(true)),
            ("sabnzbdSocksAuth", json!("999")),
            ("sabnzbdURL", json!(service.url())),
            ("qBittorrentSocksEnabled", json!(true)),
            ("qBittorrentSocksAuth", json!("999")),
            ("qBittorrentURL", json!(service.url())),
            ("lidarrSocksEnabled", json!(true)),
            ("lidarrSocksAuth", json!("999")),
            ("lidarrURL", json!(service.url())),
        ];
        let app = app_for(database, &settings).await;

        let tautulli = "/socks/tautulli/api/v2?apikey=key-of-tautulli&cmd=get_activity";
        let basic = ("authorization", "Basic dXNlcjpzZWNyZXQ=");
        assert_passed_on(&with_headers(&app, "GET", tautulli, &[basic], "").await);
        assert_passed_on(
            &call(
                &app,
                As::Guest,
                "GET",
                "/socks/sabnzbd/api?mode=queue&apikey=k",
                None,
            )
            .await,
        );
        assert_passed_on(
            &call(
                &app,
                As::Guest,
                "GET",
                "/socks/qbittorrent/api/v2/app/version",
                None,
            )
            .await,
        );
        assert_passed_on(
            &with_headers(
                &app,
                "GET",
                "/socks/lidarr/api/v1/artist",
                &[("x-api-key", "k")],
                "",
            )
            .await,
        );
        // NZBGet gets the `Authorization` header of the caller, as in PHP. The user name
        // and the password in the path go too.
        let answer = with_headers(
            &app,
            "POST",
            "/socks/nzbget/user:secret/jsonrpc",
            &[
                ("content-type", "application/json"),
                ("authorization", "Basic dXNlcjpzZWNyZXQ="),
            ],
            r#"{"method":"status"}"#,
        )
        .await;
        assert_passed_on(&answer);
        let seen = service.seen();
        let paths: Vec<&str> = seen.iter().map(|seen| seen.path.as_str()).collect();
        assert_eq!(
            paths,
            [
                "/api/v2?apikey=key-of-tautulli&cmd=get_activity",
                "/api?mode=queue&apikey=k",
                "/api/v2/app/version",
                "/api/v1/artist",
                "/user:secret/jsonrpc",
            ]
        );
        assert_eq!(seen[3].header("x-api-key"), Some("k"));
        assert_eq!(
            seen[4].header("authorization"),
            Some("Basic dXNlcjpzZWNyZXQ=")
        );
        for other in &seen[..4] {
            assert_eq!(other.header("authorization"), None, "{other:?}");
        }
        assert_eq!(seen[4].body, r#"{"method":"status"}"#);
    })
    .await;
}
