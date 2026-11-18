//! The replay runner must fail when it should. A runner that passes everything proves
//! nothing, so each way of hiding a difference is tried here and must be refused.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::{Aspect, EVERY_CASE, Replay};
use organizr_testkit::{app, seed};
use serde_json::json;

/// The replay of the `basics` group with the differences that are allowed today.
fn basics() -> Replay {
    let mut replay = Replay::group("basics").pending(
        EVERY_CASE,
        Aspect::Cookies,
        "cookies are not implemented yet",
    );
    for case in ["status", "status_with_trailing_slash"] {
        replay = replay.deviation("D-21", case, "/response/data/php", json!("n/a"));
        if cfg!(windows) {
            replay = replay.environment(
                "this test runs on Windows",
                case,
                "/response/data/os",
                json!("win"),
            );
        }
    }
    replay
}

#[tokio::test]
async fn allowed_differences_alone_let_the_replay_pass() {
    on_each_engine(async |database| {
        seed::installed(database).await;

        basics().run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Status differs")]
async fn application_that_answers_differently_fails_the_replay() {
    // A router without routes answers every request with an empty 404.
    basics().run(&Router::new()).await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Body differs")]
async fn body_that_differs_in_one_value_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The recording says "pong"; this expects another word from the application.
        let replay = basics().environment(
            "a deliberate mistake",
            "ping",
            "/response/data",
            json!("pang"),
        );

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `status`: Body differs")]
async fn difference_without_its_deviation_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // No deviation for the PHP version: the recorded value is expected as it is.
        let replay = Replay::group("basics").pending(
            EVERY_CASE,
            Aspect::Cookies,
            "cookies are not implemented yet",
        );

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Body is marked pending but now matches")]
async fn pending_marker_on_something_that_works_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().pending("ping", Aspect::Body, "not needed any more");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `rox` is marked pending but now matches")]
async fn pending_case_that_works_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().pending_case("rox", "not needed any more");

        replay.run(&app(database)).await;
    })
    .await;
}

#[test]
#[should_panic(expected = "D-999 is not an entry of docs/deviations.md")]
fn deviation_that_is_not_approved_is_refused() {
    let _ = Replay::group("basics").deviation("D-999", "ping", "/response/data", json!("x"));
}

#[tokio::test]
#[should_panic(expected = "the group `basics` has no case `no_such_case`")]
async fn marker_for_a_case_that_does_not_exist_is_refused() {
    let replay = basics().pending_case("no_such_case", "a typing mistake");

    replay.run(&Router::new()).await;
}
