//! The small routes of the PHP application: the ignored news, the sponsors, the
//! address lookup, the icon list, the shared SMTP account, the pages and the import of
//! users from a media server.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{StatusCode, header};
use axum::routing::get;
use organizr_api::AppState;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, MEDIA_SERVER_KEY};
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::{app, installed_app, installed_settings, seed, state_with};
use serde_json::{Value, json};

fn app_of(state: AppState) -> Router {
    router(state).expect("the route table is valid")
}

fn installed_state(database: &Database, changed: &[(&str, Value)]) -> AppState {
    state_with(database, installed_settings(changed))
}

async fn setting(app: &Router, key: &str) -> Value {
    let answer = call(app, As::Admin, "GET", "/config", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer.data()[key].clone()
}

/// A guest and a user get `401` from the route.
async fn is_for_co_admins(database: &Database, app: &Router, method: &str, path: &str) {
    let answer = call(app, As::Guest, method, path, None).await;
    assert_eq!(
        answer.status,
        StatusCode::UNAUTHORIZED,
        "{path}: {}",
        answer.body
    );
    assert_eq!(answer.message(), json!("Not Authorized"));
    let user = seed::logged_in(database, app, &format!("user-{}", path.len()), 4).await;
    let answer = call(app, As::User(&user), method, path, None).await;
    assert_eq!(
        answer.status,
        StatusCode::UNAUTHORIZED,
        "{path}: {}",
        answer.body
    );
}

#[tokio::test]
async fn ignored_news_ids_are_kept_once_each() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        is_for_co_admins(database, &app, "GET", "/news").await;
        is_for_co_admins(database, &app, "POST", "/news/4").await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let who = As::User(&co_admin);

        let answer = call(&app, who, "GET", "/news", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), Value::Null);
        assert_eq!(answer.headers[header::CONTENT_TYPE], "application/json");

        for id in ["4", "4", "abc", "7x"] {
            let answer = call(&app, who, "POST", &format!("/news/{id}"), None).await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.message(), json!("News id is now ignored"));
            assert_eq!(answer.headers[header::CONTENT_TYPE], "application/json");
        }
        let answer = call(&app, who, "GET", "/news", None).await;
        assert_eq!(answer.data(), json!([4, 0, 7]));

        let answer = call(&app, who, "POST", "/news/0", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("News id was not supplied"));
        assert_eq!(answer.headers[header::CONTENT_TYPE], "application/json");
    })
    .await;
}

#[tokio::test]
async fn ignored_news_id_of_an_old_install_counts_as_its_number() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = app_of(installed_state(
            database,
            &[("ignoredNewsIds", json!(["4"]))],
        ));
        let answer = call(&app, As::Admin, "POST", "/news/4", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(setting(&app, "ignoredNewsIds").await, json!(["4"]));
    })
    .await;
}

const SPONSOR_PAGE: &str = r#"<html><body><div id="sponsors">
<a href="/zed"><img src="https://avatars.example.com/u/1?s=60" alt="@Zed"></a>
<a href="/amy"><img src="https://avatars.example.com/u/2?s=60" alt="@amy"></a>
</div></body></html>"#;

async fn sponsor_services() -> StandIn {
    StandIn::start(
        Router::new()
            .route(
                "/members.json",
                get(|| async {
                    axum::Json(json!([
                        {"name": "Mia", "type": "USER", "role": "BACKER", "isActive": true},
                        {"name": "bob", "type": "USER", "role": "BACKER", "isActive": true},
                    ]))
                }),
            )
            .route(
                "/sponsors",
                get(|| async { axum::response::Html(SPONSOR_PAGE) }),
            ),
    )
    .await
}

#[tokio::test]
async fn sponsors_are_listed_for_everybody() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let services = sponsor_services().await;
        let state = installed_state(database, &[]).with_sponsors(
            &format!("{}/members.json?limit=100&offset=0", services.url()),
            &format!("{}/sponsors", services.url()),
        );
        let app = app_of(state);

        let answer = call(&app, As::Guest, "GET", "/sponsors/opencollective", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data()[0]["sortName"], "mia");
        assert_eq!(answer.data()[1]["sortName"], "bob");
        assert_eq!(services.seen()[0].path, "/members.json?limit=100&offset=0");

        let answer = call(&app, As::Guest, "GET", "/sponsors/github", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data()[0],
            json!({"name": "Zed", "sortName": "zed", "image": "https://avatars.example.com/u/1?s=200", "isActive": true, "type": "USER", "role": "BACKER"})
        );

        let answer = call(&app, As::Guest, "GET", "/sponsors/all", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), Value::Null);
        let names: Vec<Value> = answer
            .data()
            .as_array()
            .expect("a list")
            .iter()
            .map(|sponsor| sponsor["sortName"].clone())
            .collect();
        assert_eq!(names, vec![json!("amy"), json!("bob"), json!("mia"), json!("zed")]);

        // The lists are kept for a while: each service was asked once.
        let answer = call(&app, As::Guest, "GET", "/sponsors/opencollective", None).await;
        assert_eq!(answer.data()[0]["sortName"], "mia");
        let paths: Vec<String> = services.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(paths, ["/members.json?limit=100&offset=0", "/sponsors"]);
    })
    .await;
}

#[tokio::test]
async fn failed_list_of_sponsors_answers_as_php_answers() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let services = sponsor_services().await;
        // The GitHub page is missing, and Open Collective is not there.
        let state = installed_state(database, &[]).with_sponsors(
            "http://127.0.0.1:1/members.json",
            &format!("{}/missing", services.url()),
        );
        let app = app_of(state);

        let answer = call(&app, As::Guest, "GET", "/sponsors/github", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("Error connecting to Github"));
        let answer = call(&app, As::Guest, "GET", "/sponsors/opencollective", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains("127.0.0.1:1"), "{}", answer.body);

        let answer = call(&app, As::Guest, "GET", "/sponsors/all", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.json()["response"]["result"], "success");
        assert_eq!(answer.message(), json!("Error connecting to Github"));
        assert_eq!(answer.data(), Value::Null);
    })
    .await;
}

#[tokio::test]
async fn address_is_looked_up_at_ipinfo_for_a_co_admin() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let ipinfo = StandIn::start(Router::new().route(
            "/{ip}/json",
            get(
                |axum::extract::Path(ip): axum::extract::Path<String>| async move {
                    axum::Json(json!({"ip": ip, "country": "DE"}))
                },
            ),
        ))
        .await;
        let app = app_of(installed_state(database, &[]).with_ipinfo(&ipinfo.url()));
        is_for_co_admins(database, &app, "GET", "/ip/8.8.8.8").await;

        let answer = call(&app, As::Admin, "GET", "/ip/8.8.8.8", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), json!({"ip": "8.8.8.8", "country": "DE"}));
        assert_eq!(ipinfo.seen()[0].path, "/8.8.8.8/json");
        let answer = call(&app, As::Admin, "GET", "/ip/2001:db8::1", None).await;
        assert_eq!(answer.data()["ip"], "2001:db8::1");

        for wrong in ["0", "not-an-address", "8.8.8.8%2Fx"] {
            let answer = call(&app, As::Admin, "GET", &format!("/ip/{wrong}"), None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{wrong}");
            assert_eq!(answer.message(), json!("No IP Address supplied"));
        }
        assert_eq!(ipinfo.seen().len(), 2);

        let away = app_of(installed_state(database, &[]).with_ipinfo("http://127.0.0.1:1"));
        let answer = call(&away, As::Admin, "GET", "/ip/8.8.8.8", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("An error occurred"));
    })
    .await;
}

#[tokio::test]
async fn icons_are_listed_a_page_at_a_time() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        is_for_co_admins(database, &app, "GET", "/icon").await;

        let answer = call(&app, As::Admin, "GET", "/icon", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["limit"], json!(20));
        assert_eq!(data["page"], json!(1));
        assert_eq!(data["term"], Value::Null);
        assert_eq!(data["total"], json!(3324));
        assert_eq!(data["pagination"], json!({"more": true}));
        assert_eq!(data["results"].as_array().map(Vec::len), Some(20));
        assert_eq!(
            data["results"][0],
            json!({"id": "alphanumeric::A", "text": "A"})
        );

        let answer = call(
            &app,
            As::Admin,
            "GET",
            "/icon?search=COG&limit=2&page=1",
            None,
        )
        .await;
        let data = answer.data();
        assert_eq!(data["limit"], json!("2"));
        assert_eq!(data["page"], json!("1"));
        assert_eq!(data["term"], json!("COG"));
        assert!(
            data["results"]
                .as_array()
                .expect("a list")
                .iter()
                .all(|icon| icon["text"]
                    .as_str()
                    .is_some_and(|text| text.to_lowercase().contains("cog")))
        );
        let total = data["total"].as_u64().expect("a number");
        assert!(total >= 2, "{total}");

        let last = total.div_ceil(2);
        let answer = call(
            &app,
            As::Admin,
            "GET",
            &format!("/icon?search=cog&limit=2&page={last}"),
            None,
        )
        .await;
        assert_eq!(answer.data()["pagination"], json!({"more": false}));

        for wrong in ["limit=0", "page=0", "page=x"] {
            let answer = call(&app, As::Admin, "GET", &format!("/icon?{wrong}"), None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{wrong}");
        }
    })
    .await;
}

async fn smtp_service(data: Value) -> StandIn {
    StandIn::start(Router::new().route(
        "/",
        get(move || {
            let data = data.clone();
            async move { axum::Json(json!({"response": {"result": "success", "data": data}})) }
        }),
    ))
    .await
}

#[tokio::test]
async fn shared_smtp_account_sets_only_the_mail_server() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = smtp_service(json!({
            "PHPMAILER-enabled": "true",
            "PHPMAILER-smtpHost": "smtp.example.com",
            "PHPMAILER-smtpHostPort": 587,
            "PHPMAILER-smtpHostPassword": "made-up-password",
            "organizrAPI": "a-key-of-somebody-else",
            "customJava": "alert(1)",
        }))
        .await;
        let app = app_of(installed_state(database, &[]).with_organizr_api(&service.url()));
        is_for_co_admins(database, &app, "GET", "/help/smtp").await;

        // The route changes settings, so another site cannot ask it.
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let refused = send(
            &app,
            request(
                "GET",
                "/api/v2/help/smtp",
                &[("cookie", &co_admin), ("sec-fetch-site", "cross-site")],
                "",
            ),
        )
        .await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), 0);

        let answer = call(&app, As::User(&co_admin), "GET", "/help/smtp", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("SMTP activated with Organizr SMTP account")
        );
        assert_eq!(answer.data(), json!(true));
        assert_eq!(service.seen()[0].path, "/?cmd=smtp");

        assert_eq!(setting(&app, "PHPMAILER-enabled").await, json!(true));
        assert_eq!(
            setting(&app, "PHPMAILER-smtpHost").await,
            json!("smtp.example.com")
        );
        assert_eq!(setting(&app, "PHPMAILER-smtpHostPort").await, json!("587"));
        assert_eq!(
            setting(&app, "PHPMAILER-smtpHostPassword").await,
            json!("***Secure***")
        );
        assert_eq!(setting(&app, "organizrAPI").await, json!(seed::API_KEY));
        assert_eq!(setting(&app, "customJava").await, json!(""));
    })
    .await;
}

#[tokio::test]
async fn shared_smtp_account_that_cannot_be_had_changes_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let empty = smtp_service(json!({})).await;
        let app = app_of(installed_state(database, &[]).with_organizr_api(&empty.url()));
        let answer = call(&app, As::Admin, "GET", "/help/smtp", None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.data(), json!(true));

        let refusing = StandIn::start(Router::new()).await;
        let app = app_of(installed_state(database, &[]).with_organizr_api(&refusing.url()));
        let answer = call(&app, As::Admin, "GET", "/help/smtp", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.message(), Value::Null);
        assert_eq!(answer.data(), json!(false));

        let app = app_of(installed_state(database, &[]).with_organizr_api("http://127.0.0.1:1"));
        let answer = call(&app, As::Admin, "GET", "/help/smtp", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.data(), json!(false));
        assert_eq!(setting(&app, "PHPMAILER-smtpHost").await, json!(""));
    })
    .await;
}

#[tokio::test]
async fn page_names_answer_with_their_place_in_the_interface() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        let answer = call(&app, As::Guest, "GET", "/page", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        let names = answer.data();
        assert_eq!(names.as_array().map(Vec::len), Some(26));
        assert_eq!(names[0], "dependencies");
        assert_eq!(names[25], "settings_wizard");

        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        for (path, who, route) in [
            ("/page/login", As::Guest, "#/login"),
            ("/page/homepage", As::Guest, "#/"),
            ("/page/settings", As::User(&co_admin), "#/settings/general"),
            (
                "/page/Settings-Tab-Editor-Tabs",
                As::Admin,
                "#/settings/tabs",
            ),
            (
                "/page/settings_user_manage_users",
                As::Admin,
                "#/settings/users",
            ),
            ("/organizr/login", As::Guest, "#/login"),
            (
                "/organizr/settings/one/two",
                As::Admin,
                "#/settings/general",
            ),
        ] {
            let answer = call(&app, who, "GET", path, None).await;
            assert_eq!(answer.status, StatusCode::OK, "{path}: {}", answer.body);
            assert_eq!(answer.data(), json!(route), "{path}");
        }

        let answer = call(&app, As::Guest, "GET", "/page/settings", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(
            &app,
            As::User(&user),
            "GET",
            "/page/settings_tab_editor_tabs",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);

        for missing in [
            "/page/nothing",
            "/page/settings_wizard",
            "/organizr/nothing",
        ] {
            let answer = call(&app, As::Admin, "GET", missing, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{missing}");
            assert_eq!(answer.message(), json!("Page not setup"));
        }
    })
    .await;
}

#[tokio::test]
async fn pages_of_an_install_that_is_not_set_up_are_empty() {
    on_each_engine(async |database| {
        let app = app(database);
        let answer = call(&app, As::Guest, "GET", "/page/login", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), Value::Null);
        let answer = call(&app, As::Guest, "GET", "/page/wizard", None).await;
        assert_eq!(answer.data(), json!("#/"));
    })
    .await;
}

const PLEX_FRIENDS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<MediaContainer friendlyName="myPlex" size="4">
  <User id="1" username="Robin" email="robin@example.com" restricted="0"/>
  <User id="2" username="Kid" email="kid@example.com" restricted="1"/>
  <User id="3" username="cora" email="cora@example.com" restricted="0"/>
  <User id="4" username="" email="noname@example.com" restricted="0"/>
</MediaContainer>"#;

async fn plex_friends() -> StandIn {
    StandIn::start(Router::new().route(
        "/api/users",
        get(|headers: axum::http::HeaderMap| async move {
            if headers
                .get("x-plex-token")
                .and_then(|value| value.to_str().ok())
                == Some("made-up-owner-token")
            {
                (StatusCode::OK, PLEX_FRIENDS)
            } else {
                (StatusCode::UNAUTHORIZED, "")
            }
        }),
    ))
    .await
}

async fn usernames(app: &Router) -> Vec<String> {
    let answer = call(app, As::Admin, "GET", "/users", None).await;
    answer
        .data()
        .as_array()
        .expect("a list")
        .iter()
        .map(|user| user["username"].as_str().expect("a name").to_owned())
        .collect()
}

#[tokio::test]
async fn plex_friends_without_an_account_are_imported() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = plex_friends().await;
        let state = installed_state(database, &[("plexToken", json!("made-up-owner-token"))])
            .with_plex_tv(&plex.url());
        let app = app_of(state);
        is_for_co_admins(database, &app, "POST", "/users/import/plex").await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        let answer = call(
            &app,
            As::User(&co_admin),
            "POST",
            "/users/import/plex",
            Some(json!({"type": "plex"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Imported 1 users"));
        let names = usernames(&app).await;
        assert!(names.contains(&"Robin".to_owned()), "{names:?}");
        assert!(!names.contains(&"Kid".to_owned()), "{names:?}");

        let answer = call(&app, As::Admin, "POST", "/users/import/plex", None).await;
        assert_eq!(answer.message(), json!("Imported 0 users"));

        // Another type is no import, and without a token nothing is asked.
        let answer = call(&app, As::Admin, "POST", "/users/import/other", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.message(), Value::Null);
        let asked = plex.seen().len();
        let none = app_of(installed_state(database, &[]).with_plex_tv(&plex.url()));
        let answer = call(&none, As::Admin, "POST", "/users/import/plex", None).await;
        assert_eq!(answer.message(), json!("Imported 0 users"));
        assert_eq!(plex.seen().len(), asked);
    })
    .await;
}

#[tokio::test]
async fn users_of_jellyfin_and_emby_get_made_up_addresses() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let jellyfin = homepage_services::jellyfin().await;
        let emby = homepage_services::emby().await;
        let app = app_of(installed_state(
            database,
            &[
                ("jellyfinURL", json!(jellyfin.url())),
                ("jellyfinToken", json!(MEDIA_SERVER_KEY)),
                ("embyURL", json!(emby.url())),
                ("embyToken", json!(MEDIA_SERVER_KEY)),
            ],
        ));

        let answer = call(&app, As::Admin, "POST", "/users/import/jellyfin", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Imported 2 users"));
        let users = call(&app, As::Admin, "GET", "/users", None).await.data();
        let zed = users
            .as_array()
            .expect("a list")
            .iter()
            .find(|user| user["username"] == "Zed")
            .expect("Zed is imported")
            .clone();
        assert!(
            zed["email"]
                .as_str()
                .is_some_and(|email| email.ends_with("@placeholder.eml")),
            "{zed}"
        );

        // The Emby server has the same users.
        let answer = call(&app, As::Admin, "POST", "/users/import/emby", None).await;
        assert_eq!(answer.message(), json!("Imported 0 users"));
        assert_eq!(emby.seen()[0].path, "/Users");
    })
    .await;
}
