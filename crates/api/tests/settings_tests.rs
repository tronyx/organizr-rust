//! The tests of the settings pages that test no service of the homepage: a frame, a
//! folder, a cron schedule, the scheduler, the webhook for the logs, and the two
//! tests of the set-up of the PHP application.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::time::{SystemTime, UNIX_EPOCH};

use axum::http::StatusCode;
use organizr_api::router;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services;
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::{installed_app, installed_app_with, installed_settings, seed, state_with};
use serde_json::{Value, json};

#[tokio::test]
async fn only_an_admin_or_a_co_admin_runs_a_test_of_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let routes = [
            ("POST", "/test/iframe"),
            ("POST", "/test/folder"),
            ("POST", "/test/database"),
            ("POST", "/test/cron"),
            ("GET", "/test/cron"),
            ("POST", "/test/slack-logs"),
        ];
        for (method, path) in routes {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
                assert_eq!(answer.message(), json!("Not Authorized"), "{method} {path}");
            }
            let answer = call(&app, As::User(&co_admin), method, path, None).await;
            assert_ne!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
        }
    })
    .await;
}

#[tokio::test]
async fn frame_test_reads_the_header_of_the_page() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pages = homepage_services::pages().await;
        let app = installed_app(database);
        let test =
            async |body: Value| call(&app, As::Admin, "POST", "/test/iframe", Some(body)).await;
        let page = |path: &str| json!({"url": format!("{}{path}", pages.url())});

        for none in [json!({}), json!({"url": ""})] {
            let answer = test(none).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND);
            assert_eq!(answer.message(), json!("URL not supplied"));
        }
        let free = test(page("/page")).await;
        assert_eq!(free.status, StatusCode::OK, "{}", free.body);
        assert_eq!(free.message(), json!("URL approved for iFrame"));

        // A page that forbids frames, one that allows its own site only, and a
        // server that is not there.
        for refused in [
            page("/no-frames"),
            page("/same-origin"),
            json!({"url": "http://127.0.0.1:1/"}),
        ] {
            let answer = test(refused).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{}", answer.body);
            assert_eq!(answer.message(), json!("URL failed approval for iFrame"));
        }
    })
    .await;
}

#[tokio::test]
async fn tests_of_the_set_up_of_php_have_nothing_to_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        // The database is there before the wizard: the answer of PHP for that case.
        for who in [As::Guest, As::Admin] {
            let body = Some(json!({"path": "/tmp/organizr-made-up"}));
            let answer = call(&app, who, "POST", "/test/path", body).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
            assert_eq!(
                answer.message(),
                json!("Endpoint disabled as database already exists")
            );
        }
        for (driver, name) in [
            (json!({}), "sqlite3"),
            (
                json!({"driver": "MySQL", "dbHost": "db", "dbUsername": "u", "dbPassword": "p"}),
                "mysqli",
            ),
            (json!({"driver": "postgres"}), "postgre"),
        ] {
            let answer = call(&app, As::Admin, "POST", "/test/database", Some(driver)).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(
                answer.message(),
                json!(format!(
                    "Database connection test not available for driver: {name}"
                ))
            );
        }
    })
    .await;
}

#[tokio::test]
async fn folder_passes_when_it_is_there_and_is_never_made() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);
        let folder = tempfile::tempdir().expect("a folder");
        let test =
            async |body: Value| call(&app, As::Admin, "POST", "/test/folder", Some(body)).await;

        for none in [json!({}), json!({"folder": ""})] {
            let answer = test(none).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(answer.message(), json!("Folder was not supplied"));
        }
        let there = test(json!({"folder": folder.path()})).await;
        assert_eq!(there.status, StatusCode::OK, "{}", there.body);
        assert_eq!(there.message(), json!("Folder approved for logs"));

        let missing = folder.path().join("not-made");
        let file = folder.path().join("a-file");
        std::fs::write(&file, "no folder").expect("a file");
        for path in [&missing, &file] {
            let answer = test(json!({"folder": path})).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(
                answer.message(),
                json!("Folder path is not valid or permissions insufficient")
            );
        }
        // The PHP application makes the folder. This application does not.
        assert!(!missing.exists());
    })
    .await;
}

#[tokio::test]
async fn cron_schedule_is_read_and_the_scheduler_is_seen() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let state = state_with(database, installed_settings(&[]));
        let app = router(state.clone()).expect("the route table is valid");
        let schedule =
            async |text: &str| call(&app, As::Admin, "POST", "/test/cron", Some(json!(text))).await;

        // The interface of the PHP application sends the schedule as one JSON text.
        for good in ["*/5 * * * *", "30 3 * * 1-5", "@weekly"] {
            let answer = schedule(good).await;
            assert_eq!(answer.status, StatusCode::OK, "{good}: {}", answer.body);
            assert_eq!(answer.message(), json!("Schedule was validated"));
        }
        for bad in ["every day", "61 * * * *", "* * * *"] {
            let answer = schedule(bad).await;
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR, "{bad}");
            assert!(
                answer
                    .message()
                    .as_str()
                    .is_some_and(|text| !text.is_empty())
            );
        }
        // A form has the schedule as the name of its first field.
        let headers = [
            ("token", seed::API_KEY),
            ("content-type", "application/x-www-form-urlencoded"),
        ];
        let form = send(
            &app,
            request("POST", "/api/v2/test/cron", &headers, "0_4_*_*_*="),
        )
        .await;
        assert_eq!(form.status, StatusCode::OK, "{}", form.body);
        for none in [Some(json!("")), Some(json!({})), None] {
            let answer = call(&app, As::Admin, "POST", "/test/cron", none).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(answer.message(), json!("Schedule was not supplied"));
        }

        // The scheduler did not wake yet, as in a server that just started without it.
        let answer = call(&app, As::Admin, "GET", "/test/cron", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("Cron file is not setup correctly"));
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("a time after 1970")
            .as_secs();
        let now = i64::try_from(now).expect("a time that fits");
        state.scheduler_woke(now - 600);
        let answer = call(&app, As::Admin, "GET", "/test/cron", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        state.scheduler_woke(now);
        let answer = call(&app, As::Admin, "GET", "/test/cron", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Cron file is setup"));
    })
    .await;
}

#[tokio::test]
async fn test_of_the_logs_sends_one_message_to_the_webhook() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pages = homepage_services::pages().await;
        let test = async |changed: &[(&'static str, Value)]| {
            call(
                &installed_app_with(database, changed),
                As::Admin,
                "POST",
                "/test/slack-logs",
                None,
            )
            .await
        };
        let hook = format!("{}/hook", pages.url());

        let off = test(&[("slackLogWebhook", json!(hook))]).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(off.message(), json!("sendLogsToSlack is disabled"));
        let on = ("sendLogsToSlack", json!(true));
        let none = test(std::slice::from_ref(&on)).await;
        assert_eq!(none.status, StatusCode::CONFLICT);
        assert_eq!(none.message(), json!("slackLogWebhook is empty"));
        // A webhook of Slack needs a channel.
        let no_channel = test(&[on.clone(), ("slackLogWebhook", json!(hook))]).await;
        assert_eq!(no_channel.status, StatusCode::CONFLICT);
        assert_eq!(no_channel.message(), json!("slackLogWebhook is empty"));
        assert_eq!(pages.seen().len(), 0);

        let channel = ("slackLogWebHookChannel", json!("#logs"));
        let sent = test(&[
            on.clone(),
            ("slackLogWebhook", json!(hook)),
            channel.clone(),
        ])
        .await;
        assert_eq!(sent.status, StatusCode::OK, "{}", sent.body);
        assert_eq!(
            sent.message(),
            json!("Slack test connection completed - Please check Slack/Discord Channel")
        );
        let seen = pages.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].method, "POST");
        assert_eq!(seen[0].path, "/hook");
        assert_eq!(seen[0].json()["channel"], "#logs");
        assert!(
            seen[0].json()["text"]
                .as_str()
                .is_some_and(|text| text.starts_with("Warning Test"))
        );

        // A webhook that refuses the message, and one that is not there. The address
        // of a webhook is a secret, so no answer names it.
        let closed = format!("{}/closed-hook", pages.url());
        let refused = test(&[
            on.clone(),
            ("slackLogWebhook", json!(closed)),
            channel.clone(),
        ])
        .await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            refused.message(),
            json!("The webhook answered with the status 404")
        );
        let away = "http://127.0.0.1:1/made-up-secret-hook";
        let answer = test(&[on, ("slackLogWebhook", json!(away)), channel]).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !answer.body.contains("made-up-secret-hook"),
            "{}",
            answer.body
        );
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
    })
    .await;
}
