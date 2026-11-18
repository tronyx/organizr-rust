//! The Rust application must answer the recorded requests as the PHP application did.
//!
//! Each test replays one group of `parity/cases` on every database engine. See
//! `organizr_testkit::replay` for what a replay compares and how a difference is allowed.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::Replay;
use organizr_testkit::{app, installed_app, seed};
use serde_json::json;

/// The differences that every status answer has.
fn status_differences(replay: Replay, case: &str) -> Replay {
    // There is no PHP, so the PHP version has a fixed value.
    let replay = replay.deviation("D-21", case, "/response/data/php", json!("n/a"));
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

/// The cookie difference that every answer has: there is no PHP session.
fn without_php_session(replay: Replay) -> Replay {
    replay.without_cookie("D-27", "PHPSESSID")
}

#[tokio::test]
async fn fresh_install_answers_like_php() {
    on_each_engine(async |database| {
        let replay = Replay::group("fresh_install")
            .pending_case("wizard", "the set-up wizard is not implemented yet");
        let replay = without_php_session(status_differences(replay, "status"));

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn basic_routes_answer_like_php() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = Replay::group("basics");
        let replay = status_differences(replay, "status");
        let replay = status_differences(replay, "status_with_trailing_slash");
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

        replay.run(&installed_app(database)).await;
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
        // A user sees the token of the session of the request only. PHP shows the tokens
        // of all sessions of the user. Session 1 is the one of the set-up wizard, 2 is
        // browser A and 3 is browser B.
        for (case, sessions_of_others) in [
            ("me_in_browser_a", [0, 2].as_slice()),
            ("validate_token_in_the_query", &[0, 2]),
            ("validate_token_in_a_form", &[0, 2]),
            ("validate_token_of_another_browser", &[0, 2]),
            ("me_in_browser_a_after_the_delete", &[0]),
        ] {
            for index in sessions_of_others {
                replay = replay.deviation(
                    "D-02",
                    case,
                    &format!("/response/data/tokenList/{index}/token"),
                    serde_json::Value::Null,
                );
            }
        }

        replay.run(&installed_app(database)).await;
    })
    .await;
}
