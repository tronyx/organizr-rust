//! The Rust application must answer the recorded requests as the PHP application did.
//!
//! Each test replays one group of `parity/cases` on every database engine. See
//! `organizr_testkit::replay` for what a replay compares and how a difference is allowed.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::{Aspect, EVERY_CASE, Replay};
use organizr_testkit::{app, seed};
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

/// The cookie difference that every answer has for now.
fn cookies_pending(replay: Replay) -> Replay {
    replay.pending(
        EVERY_CASE,
        Aspect::Cookies,
        "the device cookie arrives with the session slice; whether to send PHPSESSID is undecided",
    )
}

#[tokio::test]
async fn fresh_install_answers_like_php() {
    on_each_engine(async |database| {
        let replay = Replay::group("fresh_install")
            .pending_case("wizard", "the set-up wizard is not implemented yet");
        let replay = cookies_pending(status_differences(replay, "status"));

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
        let replay = cookies_pending(replay);

        replay.run(&app(database)).await;
    })
    .await;
}
