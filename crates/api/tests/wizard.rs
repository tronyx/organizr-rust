//! `POST /wizard` through the real route table: what the recorded PHP answers cannot show.
//!
//! `crates/testkit/tests/parity.rs` replays a set-up that PHP accepted and the answers
//! for missing fields. These tests look at the two rules that PHP does not have, the
//! set-up code and the check of the database engine, and at what a set-up leaves behind.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_api::{AppState, router};
use organizr_auth::password::{self, PasswordCheck};
use organizr_db::{Database, Engine};
use organizr_domain::PasswordHash;
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, request, send};
use organizr_testkit::{app, seed};
use serde_json::{Value, json};

const API_KEY: &str = "abcdefghij0123456789";

/// A set-up request that is complete, with the given set-up code.
fn set_up(code: &str) -> Value {
    json!({
        "setupCode": code,
        "license": "personal",
        "api": API_KEY,
        "registrationPassword": "let me in",
        "username": "Robin",
        "password": "the password of robin",
        "email": "robin@example.com",
    })
}

async fn post_wizard(app: &Router, body: &Value, headers: &[(&str, &str)]) -> Answer {
    let mut all = vec![("content-type", "application/json")];
    all.extend_from_slice(headers);
    send(
        app,
        request("POST", "/api/v2/wizard", &all, &body.to_string()),
    )
    .await
}

async fn is_set_up(database: &Database) -> bool {
    database
        .install()
        .is_done()
        .await
        .expect("the question can be answered")
}

/// Asserts that nothing of a set-up is stored.
async fn assert_untouched(database: &Database, what: &str) {
    assert!(!is_set_up(database).await, "{what}");
    let groups = database
        .groups()
        .list()
        .await
        .expect("the groups can be read");
    assert_eq!(groups, [], "{what}");
    let settings = database
        .settings()
        .all()
        .await
        .expect("the settings can be read");
    assert_eq!(settings, [], "{what}");
}

fn with(mut body: Value, key: &str, value: Value) -> Value {
    body.as_object_mut()
        .expect("the body is an object")
        .insert(key.to_owned(), value);
    body
}

#[tokio::test]
async fn set_up_needs_the_code_from_the_server_log() {
    on_each_engine(async |database| {
        let app = app(database);
        let without_field = {
            let mut body = set_up("");
            body.as_object_mut()
                .expect("the body is an object")
                .remove("setupCode");
            body
        };
        let wrong = [
            without_field,
            set_up(""),
            set_up("PARK-TYPA-RKTZ"),
            set_up("PARK-TYPA-RKT"),
            set_up("AAAA-AAAA-AAAA"),
            with(set_up(""), "setupCode", json!(["PARK-TYPA-RKTY"])),
            with(set_up(""), "setupCode", json!(true)),
        ];

        for body in wrong {
            let answer = post_wizard(&app, &body, &[]).await;

            assert_eq!(answer.status, 403, "{body}: {}", answer.body);
            assert_eq!(
                answer.message(),
                json!("Setup code is missing or wrong. The server log shows the code"),
                "{body}"
            );
            assert_eq!(answer.data(), json!(false), "{body}");
            assert_eq!(
                answer.cookies_named("organizr_token_temp"),
                Vec::<&str>::new()
            );
            assert_untouched(database, &body.to_string()).await;
        }

        // The right code, typed with small letters and without dashes.
        let answer = post_wizard(&app, &set_up("parktyparkty"), &[]).await;
        assert_eq!(answer.status, 200, "{}", answer.body);
        assert_eq!(answer.data(), json!(true));
        assert!(is_set_up(database).await);
    })
    .await;
}

#[tokio::test]
async fn server_without_a_set_up_code_takes_no_set_up() {
    on_each_engine(async |database| {
        // A state as the server makes it for an install that is set up: without a code.
        let state = AppState::new(database.clone(), Settings::defaults(), seed::secrets());
        let app = router(state).expect("the route table is valid");

        for code in [seed::SETUP_CODE, "", "AAAA-AAAA-AAAA"] {
            let answer = post_wizard(&app, &set_up(code), &[]).await;

            assert_eq!(answer.status, 403, "{code:?}: {}", answer.body);
            assert_untouched(database, code).await;
        }
    })
    .await;
}

#[tokio::test]
async fn code_is_asked_for_before_the_fields_are_looked_at() {
    on_each_engine(async |database| {
        let app = app(database);

        // No code and no fields: the answer says nothing about the fields.
        let nothing = post_wizard(&app, &json!({}), &[]).await;
        assert_eq!(nothing.status, 403, "{}", nothing.body);

        // The right code and no fields: now the fields are named, in the order of PHP.
        let only_code = post_wizard(&app, &json!({"setupCode": seed::SETUP_CODE}), &[]).await;
        assert_eq!(only_code.status, 422, "{}", only_code.body);
        assert_eq!(only_code.message(), json!("[license] cannot be empty"));
        assert_untouched(database, "only the code").await;
    })
    .await;
}

#[tokio::test]
async fn set_up_is_refused_when_it_names_another_database_engine() {
    on_each_engine(async |database| {
        let app = app(database);
        let (this_engine, same, other) = match database.engine() {
            Engine::Sqlite => ("sqlite", ["sqlite", "sqlite3", "SQLite3"], "postgresql"),
            Engine::Postgres => (
                "postgresql",
                ["postgre", "postgres", "PostgreSQL"],
                "sqlite3",
            ),
        };

        // Another engine, and an engine that this application does not have.
        for driver in [other, "mysqli", "oracle"] {
            let body = with(set_up(seed::SETUP_CODE), "driver", json!(driver));

            let answer = post_wizard(&app, &body, &[]).await;

            assert_eq!(answer.status, 422, "{driver}: {}", answer.body);
            assert_eq!(
                answer.message(),
                json!(format!(
                    "[driver] is {driver}, but this server uses {this_engine}. Set \
                     ORGANIZR_DATABASE_URL before the first start to choose the database"
                )),
                "{driver}"
            );
            assert_eq!(answer.data(), json!(false));
            assert_untouched(database, driver).await;
        }

        // The status answer tells which engine the server uses.
        let status = send(&app, request("GET", "/api/v2/status", &[], "")).await;
        assert_eq!(status.data()["database"], json!(this_engine));
        assert_eq!(status.data()["action"], json!("wizard"));

        // The engine in use, under any of its names, is accepted. So is an empty field.
        let body = with(set_up(seed::SETUP_CODE), "driver", json!(""));
        let wrong_field = with(body, "license", json!(""));
        let answer = post_wizard(&app, &wrong_field, &[]).await;
        assert_eq!(answer.message(), json!("[license] cannot be empty"));
        for driver in same {
            let body = with(set_up("wrong"), "driver", json!(driver));
            let body = with(body, "setupCode", json!(seed::SETUP_CODE));
            let body = with(body, "email", json!(""));
            let answer = post_wizard(&app, &body, &[]).await;
            assert_eq!(
                answer.message(),
                json!("[email] cannot be empty"),
                "{driver} passes the engine check"
            );
        }
        let body = with(set_up(seed::SETUP_CODE), "driver", json!(same[0]));
        let answer = post_wizard(&app, &body, &[]).await;
        assert_eq!(answer.status, 200, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn running_application_uses_what_the_set_up_wrote() {
    on_each_engine(async |database| {
        let app = app(database);
        let before = send(
            &app,
            request("GET", "/api/v2/token/me", &[("token", API_KEY)], ""),
        )
        .await;
        assert_eq!(before.data()["username"], json!("Guest"), "no API key yet");

        let answer = post_wizard(
            &app,
            &set_up(seed::SETUP_CODE),
            &[("user-agent", "Browser/1")],
        )
        .await;
        assert_eq!(answer.status, 200, "{}", answer.body);

        // The answer logs the admin in. The cookie has the name of the new install id,
        // and scripts cannot read it.
        let stored: std::collections::BTreeMap<String, String> = database
            .settings()
            .all()
            .await
            .expect("the settings can be read")
            .into_iter()
            .map(|setting| {
                let value: String =
                    serde_json::from_str(&setting.value).expect("the stored value is JSON text");
                (setting.key, value)
            })
            .collect();
        let cookie_name = format!("organizr_token_{}", stored["uuid"]);
        let cookies = answer.cookies_named(&cookie_name);
        assert_eq!(cookies.len(), 2, "{:?}", answer.cookies);
        for cookie in &cookies {
            assert!(cookie.ends_with("; SameSite=Lax; HttpOnly"), "{cookie}");
        }
        let session = answer.cookie(&cookie_name).expect("the cookie is set");

        let me = send(
            &app,
            request(
                "GET",
                "/api/v2/token/me",
                &[("cookie", &session), ("user-agent", "Browser/1")],
                "",
            ),
        )
        .await;
        let data = me.data();
        assert_eq!(data["username"], json!("Robin"));
        assert_eq!(data["groupID"], json!(0));
        assert_eq!(data["email"], json!("robin@example.com"));
        assert_eq!(
            data["image"],
            json!("https://www.gravatar.com/avatar/d3c17ef52c397921a87c1de66f2311c2?s=100&d=mm")
        );
        assert_eq!(data["tokenList"].as_array().map(Vec::len), Some(1));

        // The API key works at once, without a restart.
        let with_key = send(
            &app,
            request("GET", "/api/v2/token/me", &[("token", API_KEY)], ""),
        )
        .await;
        assert_eq!(with_key.data()["username"], json!("Organizr API"));

        // The admin can log in with the password, which is stored as a current hash.
        let login = json!({"username": "robin", "password": "the password of robin"});
        let logged_in = send(
            &app,
            request(
                "POST",
                "/api/v2/login",
                &[("content-type", "application/json")],
                &login.to_string(),
            ),
        )
        .await;
        assert_eq!(logged_in.status, 200, "{}", logged_in.body);
        assert_eq!(logged_in.cookies_named(&cookie_name).len(), 2);

        // The registration password is stored as a hash, and the hash is sealed.
        let sealed = &stored["registrationPassword"];
        let hash = PasswordHash::new(
            seed::secrets()
                .open("registrationPassword", sealed)
                .expect("the key of the install opens the hash"),
        );
        assert!(!sealed.contains("$argon2"), "{sealed}");
        assert_eq!(
            password::verify("let me in", &hash),
            PasswordCheck::Correct { outdated: false }
        );
        assert!(!stored.values().any(|value| value == "let me in"));

        let status = send(&app, request("GET", "/api/v2/status", &[], "")).await;
        assert_eq!(status.data()["action"], json!("launch"));
    })
    .await;
}

#[tokio::test]
async fn new_install_starts_on_the_modern_layout() {
    // An install made with the wizard starts on the modern layout. An install that is
    // updated or imported from PHP keeps the default of the setting, the OG layout.
    on_each_engine(async |database| {
        let app = app(database);
        let answer = post_wizard(&app, &set_up(seed::SETUP_CODE), &[]).await;
        assert_eq!(answer.status, 200, "{}", answer.body);

        let layout = database
            .settings()
            .all()
            .await
            .expect("the settings can be read")
            .into_iter()
            .find(|setting| setting.key == "layout")
            .map(|setting| setting.value);
        assert_eq!(layout.as_deref(), Some("\"modern\""));

        let launch = send(&app, request("GET", "/api/v2/launch", &[], "")).await;
        assert_eq!(launch.status, 200, "{}", launch.body);
        assert_eq!(launch.data()["look"]["layout"], json!("modern"));
        assert_eq!(launch.data()["look"]["source"], json!("default"));
    })
    .await;
}

#[tokio::test]
async fn install_that_is_set_up_cannot_be_set_up_again() {
    on_each_engine(async |database| {
        let app = app(database);
        let first = post_wizard(&app, &set_up(seed::SETUP_CODE), &[]).await;
        assert_eq!(first.status, 200, "{}", first.body);

        // With the right code, with a wrong code and without anything: the same answer.
        let intruder = with(set_up(seed::SETUP_CODE), "username", json!("intruder"));
        for body in [intruder, set_up("AAAA-AAAA-AAAA"), json!({})] {
            let answer = post_wizard(&app, &body, &[]).await;

            assert_eq!(answer.status, 401, "{body}: {}", answer.body);
            assert_eq!(
                answer.message(),
                json!("Endpoint disabled as database already exists")
            );
            assert_eq!(answer.data(), json!(false));
        }
        assert_eq!(database.users().count().await.expect("the count works"), 1);
    })
    .await;
}

#[tokio::test]
async fn two_set_ups_at_once_make_one_install() {
    on_each_engine(async |database| {
        let app = app(database);
        let first = with(set_up(seed::SETUP_CODE), "username", json!("first"));
        let second = with(set_up(seed::SETUP_CODE), "username", json!("second"));

        let (a, b) = tokio::join!(
            post_wizard(&app, &first, &[]),
            post_wizard(&app, &second, &[])
        );

        let mut statuses = [a.status.as_u16(), b.status.as_u16()];
        statuses.sort_unstable();
        assert_eq!(statuses, [200, 401], "{} / {}", a.body, b.body);
        assert_eq!(database.users().count().await.expect("the count works"), 1);
        let groups = database
            .groups()
            .list()
            .await
            .expect("the groups can be read");
        assert_eq!(groups.len(), 6);
    })
    .await;
}

#[tokio::test]
async fn another_site_cannot_set_an_install_up() {
    on_each_engine(async |database| {
        let app = app(database);

        let answer = post_wizard(
            &app,
            &set_up(seed::SETUP_CODE),
            &[("sec-fetch-site", "cross-site")],
        )
        .await;

        assert_eq!(answer.status, 401, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("Not authorized for current Route: /api/v2/wizard")
        );
        assert_untouched(database, "a request from another site").await;
    })
    .await;
}
