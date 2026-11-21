//! `GET /database/journal` and `PUT /database/journal/{option}`: the journal mode of a
//! database on SQLite, and the answer on PostgreSQL (D-97).

#![allow(
    clippy::expect_used,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use axum::http::StatusCode;
use organizr_db::Engine;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call, request, send};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

#[tokio::test]
async fn only_admins_and_co_admins_read_and_set_the_journal_mode() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        for (method, path) in [
            ("GET", "/database/journal"),
            ("PUT", "/database/journal/WAL"),
        ] {
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
                assert_eq!(answer.message(), json!("Not Authorized"));
            }
            let answer = call(&app, As::User(&co_admin), method, path, None).await;
            assert_ne!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
        }
    })
    .await;
}

#[tokio::test]
async fn journal_mode_is_read_and_set_on_sqlite_and_refused_on_postgres() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        if database.engine() == Engine::Postgres {
            for (method, path) in [
                ("GET", "/database/journal"),
                ("PUT", "/database/journal/delete"),
            ] {
                let answer = call(&app, As::Admin, method, path, None).await;
                assert_eq!(answer.status, StatusCode::CONFLICT, "{method} {path}");
                assert_eq!(
                    answer.message(),
                    json!("The journal mode applies only to a database on SQLite")
                );
            }
            return;
        }

        // A new database has write-ahead logging.
        let answer = call(&app, As::Admin, "GET", "/database/journal", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), Value::Null);
        assert_eq!(answer.data(), json!({"journal_mode": "wal"}));

        let answer = call(&app, As::Admin, "PUT", "/database/journal/delete", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("Journal Mode updated to: DELETE"));
        assert_eq!(answer.data(), json!({"journal_mode": "delete"}));
        let answer = call(&app, As::Admin, "GET", "/database/journal", None).await;
        assert_eq!(answer.data(), json!({"journal_mode": "delete"}));
        // The application works on in that mode.
        let users = call(&app, As::Admin, "GET", "/users", None).await;
        assert_eq!(users.status, StatusCode::OK);

        let answer = call(&app, As::Admin, "PUT", "/database/journal/WAL", None).await;
        assert_eq!(answer.message(), json!("Journal Mode updated to: WAL"));
        assert_eq!(answer.data(), json!({"journal_mode": "wal"}));

        // PHP takes only these two options, and answers another one with nothing.
        let answer = call(&app, As::Admin, "PUT", "/database/journal/memory", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.message(), Value::Null);
        assert_eq!(answer.data(), Value::Null);
        let answer = call(&app, As::Admin, "GET", "/database/journal", None).await;
        assert_eq!(answer.data(), json!({"journal_mode": "wal"}));
    })
    .await;
}

#[tokio::test]
async fn change_of_the_journal_mode_from_another_site_is_refused() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let admin = seed::logged_in(database, &app, "ada", 0).await;
        let answer = send(
            &app,
            request(
                "PUT",
                "/api/v2/database/journal/DELETE",
                &[("cookie", &admin), ("sec-fetch-site", "cross-site")],
                "",
            ),
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        if database.engine() == Engine::Sqlite {
            let mode = call(&app, As::Admin, "GET", "/database/journal", None).await;
            assert_eq!(mode.data(), json!({"journal_mode": "wal"}));
        }
    })
    .await;
}
