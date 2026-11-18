//! The Rust application must answer the recorded requests as the PHP application did.
//!
//! Each test replays one group of `parity/cases` on every database engine. See
//! `organizr_testkit::replay` for what a replay compares and how a difference is allowed.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_db::{Database, Engine};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::Replay;
use organizr_testkit::{NO_LOGIN_LIMIT, app, installed_app, installed_app_with, seed};
use serde_json::{Value, json};

/// The differences that every status answer has.
fn status_differences(replay: Replay, case: &str, database: &Database) -> Replay {
    // There is no PHP, so the PHP version has a fixed value.
    let replay = replay.deviation("D-21", case, "/response/data/php", json!("n/a"));
    // The answer names the database engine, which the set-up wizard cannot choose.
    let engine = match database.engine() {
        Engine::Sqlite => "sqlite",
        Engine::Postgres => "postgresql",
    };
    let replay = replay.deviation_adds("D-46", case, "/response/data", "database", json!(engine));
    if cfg!(windows) {
        replay.environment(
            "the recording was made on Linux and this test runs on Windows",
            case,
            "/response/data/os",
            json!("win"),
        )
    } else {
        replay
    }
}

/// The differences that every launch answer has.
fn launch_differences(replay: Replay, case: &str) -> Replay {
    let version = json!(env!("CARGO_PKG_VERSION"));
    let mut replay = replay
        // There is no PHP, so the PHP version and the PHP user have fixed values.
        .deviation("D-21", case, "/response/data/status/php", json!("n/a"))
        .deviation("D-21", case, "/response/data/status/php_user", json!("n/a"))
        .environment(
            "the recording was made with Organizr 2.1.5000; this is another version",
            case,
            "/response/data/version",
            version.clone(),
        )
        .environment(
            "the recording was made with Organizr 2.1.5000; this is another version",
            case,
            "/response/data/status/version",
            version,
        );
    if cfg!(windows) {
        replay = replay.environment(
            "the recording was made on Linux and this test runs on Windows",
            case,
            "/response/data/status/os",
            json!("win"),
        );
    }
    for (part, reason) in [
        (
            "settings",
            "the settings of the front end are not built yet",
        ),
        ("plugins", "the plugins are not built yet"),
        ("sso", "the single sign-on is not built yet"),
        (
            "status/userConfigPath",
            "waits for a decision: PHP sends the path of its configuration file",
        ),
    ] {
        replay = replay.pending_at(case, &format!("/response/data/{part}"), reason);
    }
    replay
}

/// The cookie difference that every answer has: there is no PHP session.
fn without_php_session(replay: Replay) -> Replay {
    replay.without_cookie("D-27", "PHPSESSID")
}

#[tokio::test]
async fn fresh_install_answers_like_php() {
    on_each_engine(async |database| {
        let mut replay = without_php_session(Replay::group("fresh_install"));
        for case in [
            "status",
            "status_before_the_set_up",
            "status_after_the_set_up",
        ] {
            replay = status_differences(replay, case, database);
        }
        // The admin sees no token, and which session is the one of the request (D-18).
        // The only session is the one that the set-up started.
        let me = "me_after_the_set_up";
        let session = "/response/data/tokenList/0";
        replay = replay
            .deviation("D-18", me, "/response/data/token", Value::Null)
            .deviation("D-18", me, &format!("{session}/token"), Value::Null)
            .deviation_adds("D-18", me, session, "current", json!(true));

        for case in [
            "launch_before_the_set_up",
            "launch_as_guest",
            "launch_as_guest_with_a_trailing_slash",
            "launch_as_admin",
        ] {
            replay = launch_differences(replay, case);
        }
        // The admin in the launch answer, as in `me_after_the_set_up` (D-18).
        let launch = "launch_as_admin";
        let session = "/response/data/user/tokenList/0";
        replay = replay
            .deviation("D-18", launch, "/response/data/user/token", Value::Null)
            .deviation("D-18", launch, &format!("{session}/token"), Value::Null)
            .deviation_adds("D-18", launch, session, "current", json!(true));
        // The one tab an admin sees after the set-up is "Settings". The set-up of PHP
        // leaves these switches empty (`null`). Here a switch is on or off.
        for switch in ["splash", "ping", "timeout", "preload"] {
            replay = replay.pending_at(
                launch,
                &format!("/response/data/tabs/0/{switch}"),
                "waits for a decision: a switch that was never set is `null` in PHP and 0 here",
            );
        }

        // The application starts on a database without an install. The recorded
        // requests carry the set-up code of the test applications, which PHP ignores.
        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn basic_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = Replay::group("basics");
        let replay = status_differences(replay, "status", database);
        let replay = status_differences(replay, "status_with_trailing_slash", database);
        let replay = without_php_session(replay);

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn login_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("login"));
        // The group fails more logins from one address than the login limit allows. PHP
        // has no limit on the server; the group `lockout` is about the limit.
        let app = installed_app_with(database, &[("loginAttempts", json!(NO_LOGIN_LIMIT))]);

        replay.run(&app).await;
    })
    .await;
}

#[tokio::test]
async fn logout_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("logout"));

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn session_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        seed::wizard_session(database).await;
        let mut replay = without_php_session(Replay::group("session"));
        // Session 1 is the one of the set-up wizard, 2 is browser A and 3 is browser B.
        //
        // `GET /token/me` holds no token, and says which session is the one of the
        // request. PHP shows the tokens of all sessions of the user.
        for (case, sessions, current) in [
            ("me_in_browser_a", 3, 1),
            ("me_in_browser_a_after_the_delete", 2, 1),
        ] {
            replay = replay.deviation("D-18", case, "/response/data/token", Value::Null);
            for index in 0..sessions {
                let entry = format!("/response/data/tokenList/{index}");
                replay = replay
                    .deviation("D-18", case, &format!("{entry}/token"), Value::Null)
                    .deviation_adds("D-18", case, &entry, "current", json!(index == current));
            }
        }
        // `POST /token/validate` gives back the token that the caller sent, but not the
        // tokens of the other sessions of the user.
        for case in [
            "validate_token_in_the_query",
            "validate_token_in_a_form",
            "validate_token_of_another_browser",
        ] {
            for index in [0, 2] {
                let token = format!("/response/data/tokenList/{index}/token");
                replay = replay.deviation("D-02", case, &token, Value::Null);
            }
        }

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn login_limit_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // PHP remembers a lock-out in a cookie of the browser. Here the server remembers.
        let replay =
            without_php_session(Replay::group("lockout")).without_cookie("D-04", "lockout");

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn api_key_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = without_php_session(Replay::group("api_key"));

        replay.run(&installed_app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn registration_answers_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The registration password of the recording, stored as the set-up stores it.
        let hash = organizr_auth::password::hash("let-new-users-in")
            .expect("the registration password can be hashed");
        let app = installed_app_with(database, &[("registrationPassword", json!(hash.expose()))]);
        let replay = without_php_session(Replay::group("register"));

        replay.run(&app).await;
    })
    .await;
}
