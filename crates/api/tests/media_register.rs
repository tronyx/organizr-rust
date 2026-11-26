//! `POST /plex/register` and `POST /emby/register`: a new account on Plex or on Emby
//! for a person with an invite code, against stand-ins for `plex.tv` and Emby.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{NO_LOGIN_LIMIT, installed_settings, seed, state_with};
use serde_json::{Value, json};

const OWNER_TOKEN: &str = "made-up-owner-token";
const SERVER_ID: &str = "made-up-server";
const EMBY_KEY: &str = "made-up-emby-admin-key";
const PASSWORD: &str = "the secret password of sam";

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

async fn plex_join(axum::Form(form): axum::Form<Vec<(String, String)>>) -> Response {
    let username = form
        .iter()
        .find(|(key, _)| key == "username")
        .map(|(_, value)| value.as_str());
    match username {
        Some("taken") => (
            StatusCode::UNPROCESSABLE_ENTITY,
            axum::Json(json!({"errors": [
                {"code": 1001, "message": "has already been taken", "field": "username"}
            ]})),
        )
            .into_response(),
        Some("broken") => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        _ => (StatusCode::CREATED, axum::Json(json!({"id": 9}))).into_response(),
    }
}

async fn plex_share(Path(server): Path<String>, headers: HeaderMap) -> Response {
    if header(&headers, "x-plex-token") != Some(OWNER_TOKEN) {
        return (StatusCode::UNAUTHORIZED, "Invalid token").into_response();
    }
    if server != SERVER_ID {
        return StatusCode::NOT_FOUND.into_response();
    }
    axum::Json(json!({"id": 3})).into_response()
}

async fn plex_tv() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/api/v2/users", post(plex_join))
            .route("/api/servers/{server}/shared_servers/", post(plex_share)),
    )
    .await
}

fn is_admin(headers: &HeaderMap) -> bool {
    header(headers, "x-emby-token") == Some(EMBY_KEY)
}

async fn emby_new(headers: HeaderMap, Query(query): Query<Vec<(String, String)>>) -> Response {
    if !is_admin(&headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let name = query
        .iter()
        .find(|(key, _)| key == "name")
        .map_or("", |(_, value)| value.as_str());
    if name == "broken" {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    axum::Json(json!({"Id": format!("id-{name}"), "Name": name})).into_response()
}

async fn emby_password(Path(id): Path<String>, headers: HeaderMap) -> Response {
    let sent = header(&headers, "x-emby-authorization").unwrap_or_default();
    if !sent.contains("Token=\"made-up-user-token\"") || id == "id-badpass" {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn emby_admin_only(headers: HeaderMap) -> Response {
    if is_admin(&headers) {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn emby() -> StandIn {
    StandIn::start(
        Router::new()
            .route("/emby/Users/New", post(emby_new))
            .route(
                "/emby/Users/AuthenticateByName",
                post(|| async { axum::Json(json!({"AccessToken": "made-up-user-token"})) }),
            )
            .route("/emby/Users/{id}/Password", post(emby_password))
            .route("/emby/Users/{id}/Policy", post(emby_admin_only))
            .route("/emby/Users/{id}/Connect/Link", post(|| async { "{}" }))
            .route("/emby/Users/{id}", delete(emby_admin_only))
            .route(
                "/emby/Users",
                get(|headers: HeaderMap| async move {
                    if !is_admin(&headers) {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    axum::Json(json!([
                        {"Id": "1", "Name": "Owner", "Policy": {"IsAdministrator": true}},
                        {"Id": "2", "Name": "Template", "Policy": {
                            "IsAdministrator": false,
                            "EnableAllFolders": false,
                            "AuthenticationProviderId": "made-up-provider",
                            "InvalidLoginAttemptCount": 2,
                            "DisablePremiumFeatures": false
                        }},
                    ]))
                    .into_response()
                }),
            ),
    )
    .await
}

fn settings(more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        ("INVITES-enabled", json!(true)),
        ("plexToken", json!(OWNER_TOKEN)),
        ("plexID", json!(SERVER_ID)),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ];
    settings.extend(more.iter().cloned());
    settings
}

fn app_with(database: &Database, plex: &str, more: &[(&'static str, Value)]) -> Router {
    let state = state_with(database, installed_settings(&settings(more))).with_plex_tv(plex);
    router(state).expect("the route table is valid")
}

async fn make_invite(app: &Router, code: &str) {
    let body = json!({"code": code, "username": "Sam", "email": "sam@example.com"});
    let made = call(app, As::Admin, "POST", "/plugins/invites", Some(body)).await;
    assert_eq!(made.status, StatusCode::OK, "{}", made.body);
}

/// `Yes` while the code can be used, then `No`.
async fn valid(app: &Router, code: &str) -> String {
    let list = call(app, As::Admin, "GET", "/plugins/invites", None).await;
    list.data()
        .as_array()
        .expect("a list")
        .iter()
        .find(|row| row["code"] == code)
        .expect("the invite is listed")["valid"]
        .as_str()
        .expect("a text")
        .to_owned()
}

async fn register(app: &Router, route: &str, username: &str, code: Option<&str>) -> Answer {
    let mut body = json!({"username": username, "email": "sam@example.com", "password": PASSWORD});
    if let Some(code) = code {
        body["code"] = json!(code);
    }
    call(app, As::Guest, "POST", route, Some(body)).await
}

fn has_no_secret(answer: &Answer) {
    for secret in [PASSWORD, EMBY_KEY, OWNER_TOKEN, "127.0.0.1:1"] {
        assert!(!answer.body.contains(secret), "{}", answer.body);
    }
}

#[tokio::test]
async fn plex_account_needs_a_usable_code_and_uses_it_up() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = plex_tv().await;
        let app = app_with(database, &plex.url(), &[]);
        make_invite(&app, "PLEX01").await;

        // The fields are checked first, as in PHP.
        let answer = call(
            &app,
            As::Guest,
            "POST",
            "/plex/register",
            Some(json!({"code": "PLEX01"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Username not supplied"));

        for code in [None, Some(""), Some("WRONG1")] {
            let answer = register(&app, "/plex/register", "sam", code).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{code:?}");
            assert_eq!(answer.message(), json!("Code is invalid"));
            has_no_secret(&answer);
        }
        assert_eq!(plex.seen().len(), 0);

        let answer = register(&app, "/plex/register", "sam", Some("plex01")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("User has joined Plex"));
        let seen = plex.seen();
        assert_eq!(seen[0].path, "/api/v2/users");
        assert_eq!(seen[0].form()["password"], PASSWORD);
        assert_eq!(
            seen[0].header("x-plex-client-identifier"),
            Some(seed::INSTALL_ID)
        );
        assert_eq!(
            seen[1].path,
            format!("/api/servers/{SERVER_ID}/shared_servers/")
        );
        assert_eq!(
            seen[1].json()["shared_server"]["invited_email"],
            "sam@example.com"
        );
        assert_eq!(valid(&app, "PLEX01").await, "No");

        let again = register(&app, "/plex/register", "sam2", Some("PLEX01")).await;
        assert_eq!(again.status, StatusCode::UNAUTHORIZED);
        assert_eq!(again.message(), json!("Code is invalid"));
        assert_eq!(plex.seen().len(), 2);
    })
    .await;
}

#[tokio::test]
async fn failed_plex_account_keeps_the_code() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = plex_tv().await;
        let app = app_with(database, &plex.url(), &[]);
        make_invite(&app, "PLEX02").await;

        let answer = register(&app, "/plex/register", "taken", Some("PLEX02")).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            answer.message(),
            json!("[Plex.tv Error: has already been taken for field: (username)]")
        );
        has_no_secret(&answer);
        assert_eq!(valid(&app, "PLEX02").await, "Yes");

        let answer = register(&app, "/plex/register", "broken", Some("PLEX02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("An Error Occurred"));
        assert_eq!(valid(&app, "PLEX02").await, "Yes");

        let away = app_with(database, "http://127.0.0.1:1", &[]);
        let answer = register(&away, "/plex/register", "sam", Some("PLEX02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        has_no_secret(&answer);
        assert_eq!(valid(&app, "PLEX02").await, "Yes");

        // The account is made, but the share fails: the code stays for the share.
        let other = app_with(
            database,
            &plex.url(),
            &[("plexID", json!("another-server"))],
        );
        let answer = register(&other, "/plex/register", "sam", Some("PLEX02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("User has joined Plex, but has no access yet: Libraries not setup correct")
        );
        assert_eq!(valid(&app, "PLEX02").await, "Yes");

        // The plugin is off.
        let off = app_with(database, &plex.url(), &[("INVITES-enabled", json!(false))]);
        let answer = register(&off, "/plex/register", "sam", Some("PLEX02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("INVITES-enabled module is not enabled")
        );
    })
    .await;
}

#[tokio::test]
async fn wrong_and_used_codes_are_counted() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = plex_tv().await;
        let app = app_with(database, &plex.url(), &[("loginAttempts", json!("3"))]);
        make_invite(&app, "USED01").await;
        let answer = register(&app, "/plex/register", "sam", Some("USED01")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let wrong = register(&app, "/plex/register", "sam", Some("NOPE01")).await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        let used = register(&app, "/emby/register", "sam", Some("USED01")).await;
        assert_eq!(used.status, StatusCode::UNAUTHORIZED);
        let third = register(&app, "/plex/register", "sam", Some("USED01")).await;
        assert_eq!(third.status, StatusCode::FORBIDDEN);
        assert_eq!(third.message(), json!("User is locked out"));
    })
    .await;
}

#[tokio::test]
async fn emby_registration_needs_invites_for_emby() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = emby().await;
        // The invites are for Plex, and the Emby item of the homepage has its key (D-149).
        let app = app_with(
            database,
            "http://127.0.0.1:1",
            &[
                ("INVITES-type-include", json!("plex")),
                ("embyURL", json!(server.url())),
                ("embyToken", json!(EMBY_KEY)),
            ],
        );
        make_invite(&app, "PLEX01").await;

        let answer = register(&app, "/emby/register", "sam", Some("PLEX01")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT, "{}", answer.body);
        assert_eq!(answer.message(), json!("Invites are not set up for Emby"));
        assert_eq!(server.seen().len(), 0);
        // The code is still there for its Plex account.
        assert_eq!(valid(&app, "PLEX01").await, "Yes");
    })
    .await;
}

#[tokio::test]
async fn emby_user_gets_the_password_and_the_policy_of_the_template() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = emby().await;
        let app = app_with(
            database,
            "http://127.0.0.1:1",
            &[
                ("INVITES-type-include", json!("emby")),
                ("INVITES-EmbyTemplate", json!("Template")),
                ("embyURL", json!(server.url())),
                ("embyToken", json!(EMBY_KEY)),
            ],
        );
        make_invite(&app, "EMBY01").await;

        let answer = call(
            &app,
            As::Guest,
            "POST",
            "/emby/register",
            Some(json!({"username": "sam", "code": "EMBY01"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("Email not supplied"));
        let answer = register(&app, "/emby/register", "sam", Some("WRONG1")).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(server.seen().len(), 0);

        let answer = register(&app, "/emby/register", "sam", Some("EMBY01")).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("User has joined Emby"));
        assert_eq!(valid(&app, "EMBY01").await, "No");

        let seen = server.seen();
        let paths: Vec<&str> = seen.iter().map(|seen| seen.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "/emby/Users/New?name=sam",
                "/emby/Users/AuthenticateByName",
                "/emby/Users/id-sam/Password",
                "/emby/Users",
                "/emby/Users/id-sam/Policy",
                "/emby/Users/id-sam/Connect/Link",
            ]
        );
        assert!(seen.iter().all(|seen| !seen.path.contains(EMBY_KEY)));
        assert_eq!(seen[1].json(), json!({"Pw": "", "Username": "sam"}));
        assert_eq!(seen[2].json()["NewPw"], PASSWORD);
        assert_eq!(
            seen[4].json(),
            json!({"IsAdministrator": false, "EnableAllFolders": false})
        );
        assert_eq!(
            seen[5].json(),
            json!({"ConnectUsername": "sam@example.com"})
        );
    })
    .await;
}

#[tokio::test]
async fn failed_emby_user_keeps_the_code_and_leaves_no_user_behind() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let server = emby().await;
        let more = [
            ("INVITES-type-include", json!("emby")),
            ("embyURL", json!(server.url())),
            ("embyToken", json!(EMBY_KEY)),
        ];
        let app = app_with(database, "http://127.0.0.1:1", &more);
        make_invite(&app, "EMBY02").await;

        let answer = register(&app, "/emby/register", "broken", Some("EMBY02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("the Emby server refused to make the user")
        );
        has_no_secret(&answer);
        assert_eq!(valid(&app, "EMBY02").await, "Yes");

        let answer = register(&app, "/emby/register", "badpass", Some("EMBY02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("the Emby server refused to set the password")
        );
        has_no_secret(&answer);
        assert_eq!(valid(&app, "EMBY02").await, "Yes");
        let deleted = server
            .seen()
            .into_iter()
            .filter(|seen| seen.method == "DELETE")
            .map(|seen| seen.path)
            .collect::<Vec<_>>();
        assert_eq!(deleted, vec!["/emby/Users/id-badpass".to_owned()]);

        let away = app_with(
            database,
            "http://127.0.0.1:1",
            &[
                ("INVITES-type-include", json!("emby")),
                ("embyURL", json!("http://127.0.0.1:1")),
                ("embyToken", json!(EMBY_KEY)),
            ],
        );
        let answer = register(&away, "/emby/register", "sam", Some("EMBY02")).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        has_no_secret(&answer);
        assert_eq!(valid(&app, "EMBY02").await, "Yes");

        let unset = app_with(
            database,
            "http://127.0.0.1:1",
            &[("INVITES-type-include", json!("emby"))],
        );
        let answer = register(&unset, "/emby/register", "sam", Some("EMBY02")).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Emby URL/Token not set"));
    })
    .await;
}
