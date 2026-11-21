//! The chat plugin: its messages, its stream of new messages and its settings (D-44,
//! D-95).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use std::time::Duration;

use axum::http::{StatusCode, header};
use http_body_util::BodyExt;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};
use tower::ServiceExt;

/// The settings of an install with the chat on, for the given level.
fn chat_on(level: &str) -> Vec<(&'static str, Value)> {
    vec![
        ("CHAT-enabled", json!(true)),
        ("CHAT-Auth-include", json!(level)),
    ]
}

#[tokio::test]
async fn the_chat_answers_only_when_it_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        for (method, path) in [
            ("GET", "/plugins/chat/message"),
            ("POST", "/plugins/chat/message"),
            ("GET", "/plugins/chat/events"),
        ] {
            let body = (method == "POST").then(|| json!({"message": "hi"}));
            let answer = call(&app, As::Admin, method, path, body).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{method} {path}");
            assert_eq!(
                answer.message(),
                json!("CHAT-enabled module is not enabled")
            );
        }
    })
    .await;
}

#[tokio::test]
async fn the_chat_is_for_the_level_of_the_setting_and_never_for_guests() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("1"));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(&app, As::User(&user), "GET", "/plugins/chat/message", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);

        let app = installed_app_with(database, &chat_on("999"));
        for (method, path) in [
            ("GET", "/plugins/chat/message"),
            ("POST", "/plugins/chat/message"),
            ("GET", "/plugins/chat/events"),
        ] {
            let body = (method == "POST").then(|| json!({"message": "hi"}));
            let answer = call(&app, As::Guest, method, path, body).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            assert_eq!(answer.message(), json!("Not Authorized"));
        }
        let answer = call(&app, As::User(&user), "GET", "/plugins/chat/message", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn messages_are_plain_text_and_come_back_newest_first() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("4"));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        for message in ["first", "<b>bold</b> & \"quoted\""] {
            let answer = call(
                &app,
                As::User(&user),
                "POST",
                "/plugins/chat/message",
                Some(json!({ "message": message })),
            )
            .await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.message(), json!("Chat message accepted"));
        }
        let answer = call(&app, As::User(&user), "GET", "/plugins/chat/message", None).await;
        let data = answer.data();
        let messages = data.as_array().expect("the data is a list");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["message"], json!("<b>bold</b> & \"quoted\""));
        assert_eq!(messages[1]["message"], json!("first"));
        assert_eq!(messages[0]["username"], json!("uma"));
        let fields: Vec<&String> = messages[0].as_object().expect("an object").keys().collect();
        assert_eq!(fields, ["username", "gravatar", "uid", "date", "message"]);
        assert!(!answer.body.contains("\"ip\""), "{}", answer.body);
    })
    .await;
}

#[tokio::test]
async fn a_message_must_have_text_within_the_limit() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("1"));
        for body in [json!({}), json!({"message": ""}), json!({"message": "0"})] {
            let answer = call(&app, As::Admin, "POST", "/plugins/chat/message", Some(body)).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(answer.message(), json!("No message supplied"));
        }
        let long = "x".repeat(1001);
        let answer = call(
            &app,
            As::Admin,
            "POST",
            "/plugins/chat/message",
            Some(json!({ "message": long })),
        )
        .await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(answer.message(), json!("String is over limit of: 1000"));
        let answer = call(&app, As::Admin, "GET", "/plugins/chat/message", None).await;
        assert_eq!(answer.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn a_sender_who_sends_too_fast_is_held_back() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("4"));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let other = seed::logged_in(database, &app, "olga", 4).await;
        let message = || Some(json!({"message": "hi"}));
        for _ in 0..10 {
            let answer = call(
                &app,
                As::User(&user),
                "POST",
                "/plugins/chat/message",
                message(),
            )
            .await;
            assert_eq!(answer.status, StatusCode::OK);
        }
        let answer = call(
            &app,
            As::User(&user),
            "POST",
            "/plugins/chat/message",
            message(),
        )
        .await;
        assert_eq!(answer.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            answer.message(),
            json!("Too many chat messages. Wait a moment, then send again.")
        );
        let answer = call(
            &app,
            As::User(&other),
            "POST",
            "/plugins/chat/message",
            message(),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn a_message_from_another_site_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("4"));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = send(
            &app,
            request(
                "POST",
                "/api/v2/plugins/chat/message",
                &[
                    ("cookie", &user),
                    ("sec-fetch-site", "cross-site"),
                    ("content-type", "application/json"),
                ],
                r#"{"message":"hi"}"#,
            ),
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.body);
        let answer = call(&app, As::User(&user), "GET", "/plugins/chat/message", None).await;
        assert_eq!(answer.data(), json!([]));
    })
    .await;
}

#[tokio::test]
async fn a_new_message_reaches_the_stream_of_events() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &chat_on("4"));
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let response = app
            .clone()
            .oneshot(request(
                "GET",
                "/api/v2/plugins/chat/events",
                &[("cookie", &user)],
                "",
            ))
            .await
            .expect("the router answers");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
        let mut body = response.into_body();

        let answer = call(
            &app,
            As::User(&user),
            "POST",
            "/plugins/chat/message",
            Some(json!({"message": "<i>live</i>"})),
        )
        .await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let frame = tokio::time::timeout(Duration::from_secs(5), body.frame())
            .await
            .expect("an event comes")
            .expect("the stream is open")
            .expect("the frame can be read");
        let text = String::from_utf8(frame.into_data().expect("a data frame").to_vec())
            .expect("the event is text");
        assert!(text.starts_with("event: message\n"), "{text}");
        let data = text
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("the event has data");
        let message: Value = serde_json::from_str(data).expect("the data is JSON");
        assert_eq!(message["message"], json!("<i>live</i>"));
        assert_eq!(message["username"], json!("uma"));
    })
    .await;
}

#[tokio::test]
async fn the_settings_of_the_chat_are_for_admins_without_pusher() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(
            database,
            &[
                ("CHAT-secret", json!("pusher-secret")),
                ("CHAT-authKey-include", json!("pusher-key")),
            ],
        );
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let answer = call(&app, As::User(&user), "GET", "/plugins/chat/settings", None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        let answer = call(&app, As::Admin, "GET", "/plugins/chat/settings", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        let names: Vec<&str> = data["Options"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|item| item["name"].as_str().expect("a name"))
            .collect();
        assert_eq!(names, ["CHAT-Auth-include", "CHAT-messageLoadLimit"]);
        assert_eq!(data["Options"][0]["value"], json!("1"));
        assert!(
            data["Options"][0]["options"]
                .as_array()
                .expect("a list")
                .contains(&json!({"name": "Admin", "value": 0}))
        );
        for secret in ["pusher-secret", "pusher-key", "custom"] {
            assert!(!answer.body.contains(secret), "{secret}");
        }
    })
    .await;
}
