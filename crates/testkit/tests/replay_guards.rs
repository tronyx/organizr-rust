//! The replay runner must fail when it should. A runner that passes everything proves
//! nothing, so each way of hiding a difference is tried here and must be refused.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_db::Engine;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::replay::{Aspect, EVERY_CASE, Replay};
use organizr_testkit::{app, seed};
use serde_json::json;

/// The replay of the `basics` group with the differences that are allowed today, for an
/// application on SQLite. A test that must fail does so on SQLite, which is tried first.
fn basics() -> Replay {
    basics_on(Engine::Sqlite)
}

/// The replay of the `basics` group with the differences that are allowed today, for an
/// application on the given engine.
fn basics_on(engine: Engine) -> Replay {
    let engine = match engine {
        Engine::Sqlite => "sqlite",
        Engine::Postgres => "postgresql",
    };
    let mut replay = Replay::group("basics").without_cookie("D-27", "PHPSESSID");
    for case in ["status", "status_with_trailing_slash"] {
        replay = replay.deviation("D-21", case, "/response/data/php", json!("n/a"));
        replay = replay.deviation_adds("D-46", case, "/response/data", "database", json!(engine));
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

        basics_on(database.engine()).run(&app(database)).await;
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
        let replay = Replay::group("basics").without_cookie("D-27", "PHPSESSID");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Cookies differ")]
async fn cookie_that_php_sets_and_no_deviation_removes_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // Without D-27 the PHP session cookie is expected, and the application sends none.
        let mut replay = Replay::group("basics");
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

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Cookies is marked pending but now matches")]
async fn pending_marker_on_cookies_that_match_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().pending(EVERY_CASE, Aspect::Cookies, "not needed any more");

        replay.run(&app(database)).await;
    })
    .await;
}

#[test]
#[should_panic(expected = "D-999 is not an entry of docs/deviations.md")]
fn cookie_deviation_that_is_not_approved_is_refused() {
    let _ = Replay::group("basics").without_cookie("D-999", "PHPSESSID");
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

#[tokio::test]
#[should_panic(expected = "case `status`: Body differs")]
async fn key_that_the_application_does_not_send_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // An approved deviation adds a key, but the application does not send it.
        let replay =
            basics().deviation_adds("D-21", "status", "/response/data", "made_up", json!(true));

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "already has the key `php`")]
async fn added_key_cannot_replace_a_recorded_value() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay =
            basics().deviation_adds("D-21", "status", "/response/data", "php", json!("n/a"));

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "D-99 is not an entry of docs/deviations.md")]
async fn added_key_needs_an_approved_deviation() {
    let _ = basics().deviation_adds("D-99", "status", "/response/data", "made_up", json!(true));
}

#[tokio::test]
async fn pending_part_of_a_body_lets_the_rest_be_compared() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The title of the `rox` answer is "not built yet": the application has it,
        // but the marker names another value than the recording for the test.
        let replay = basics_on(database.engine())
            .environment(
                "a deliberate mistake",
                "rox",
                "/response/message",
                json!("another greeting"),
            )
            .pending_at("rox", "/response/message", "a part that differs");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `rox`: Body differs")]
async fn difference_next_to_a_pending_part_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics()
            .environment(
                "a deliberate mistake",
                "rox",
                "/response/message",
                json!("another greeting"),
            )
            .pending_at("rox", "/response/message", "a part that differs")
            // The part next to it differs too, and nothing allows that.
            .environment(
                "a deliberate mistake",
                "rox",
                "/response/data",
                json!("https://example.com/"),
            );

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "the part at `/response/message` is marked pending but now matches")]
async fn pending_part_that_matches_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().pending_at("rox", "/response/message", "not needed any more");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "the recorded body has nothing at `/response/no_such_key`")]
async fn pending_part_that_the_recording_does_not_have_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().pending_at("rox", "/response/no_such_key", "a typing mistake");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "the group `basics` has no case `no_such_case`")]
async fn pending_part_of_a_case_that_does_not_exist_is_refused() {
    let replay = basics().pending_at("no_such_case", "/response/data", "a typing mistake");

    replay.run(&Router::new()).await;
}

#[tokio::test]
async fn value_for_every_null_under_a_key_is_expected() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The message of the `ping` answer is `null` in the recording. The application
        // sends `null` too, so the second edit puts the expected value back.
        let replay = basics_on(database.engine())
            .deviation_for_nulls("D-21", "ping", &["message"], json!("a message"))
            .environment(
                "put back for the test",
                "ping",
                "/response/message",
                json!(null),
            );

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: Body differs")]
async fn answer_that_keeps_the_null_fails_the_replay() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().deviation_for_nulls("D-21", "ping", &["message"], json!(0));

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `ping`: the recorded body has no `null` under [\"data\"]")]
async fn value_for_a_null_that_the_recording_does_not_have_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().deviation_for_nulls("D-21", "ping", &["data"], json!(0));

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "D-99 is not an entry of docs/deviations.md")]
async fn value_for_nulls_needs_an_approved_deviation() {
    let _ = basics().deviation_for_nulls("D-99", "ping", &["message"], json!(0));
}

#[tokio::test]
#[should_panic(expected = "the group `basics` has no case `no_such_case`")]
async fn value_for_nulls_of_a_case_that_does_not_exist_is_refused() {
    let replay = basics().deviation_for_nulls("D-21", "no_such_case", &["message"], json!(0));

    replay.run(&Router::new()).await;
}

#[tokio::test]
#[should_panic(expected = "case `rox`: Body differs")]
async fn key_that_the_answer_must_not_have_fails_the_replay_when_it_is_there() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The application sends the message, so the answer has one key too many.
        let replay = basics().deviation_without("D-21", "rox", "/response/message");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
async fn key_that_the_answer_does_not_have_is_expected_to_be_absent() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The first edit gives the recording a key that the application does not send.
        // The second one says that the key is not expected.
        let replay = basics_on(database.engine())
            .deviation_adds("D-21", "rox", "/response", "extra", json!(1))
            .deviation_without("D-21", "rox", "/response/extra");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `rox`: the recorded body has nothing at `/response/no_such_key`")]
async fn absent_key_that_the_recording_does_not_have_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().deviation_without("D-21", "rox", "/response/no_such_key");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "D-99 is not an entry of docs/deviations.md")]
async fn absent_key_needs_an_approved_deviation() {
    let _ = basics().deviation_without("D-99", "rox", "/response/message");
}

#[tokio::test]
#[should_panic(expected = "case `rox`: Body differs")]
async fn keys_that_are_expected_in_sorted_order_fail_the_replay_in_another_order() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The application sends `result`, `message`, `data`, as PHP does.
        let replay = basics().deviation_sorts_keys("D-21", "rox", "/response");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(
    expected = "case `rox`: the recorded keys at `/response` are sorted; remove the marker"
)]
async fn sorted_keys_marker_on_keys_that_are_sorted_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // The first marker sorts the keys, so the second one finds them sorted.
        let replay = basics()
            .deviation_sorts_keys("D-21", "rox", "/response")
            .deviation_sorts_keys("D-21", "rox", "/response");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "case `rox`: the recorded body has no object at `/response/message`")]
async fn sorted_keys_marker_on_something_that_is_no_object_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let replay = basics().deviation_sorts_keys("D-21", "rox", "/response/message");

        replay.run(&app(database)).await;
    })
    .await;
}

#[tokio::test]
#[should_panic(expected = "D-99 is not an entry of docs/deviations.md")]
async fn sorted_keys_need_an_approved_deviation() {
    let _ = basics().deviation_sorts_keys("D-99", "rox", "/response");
}
