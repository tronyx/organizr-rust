//! `GET /log[/{number}[/{trace_id}]]` and `DELETE /log[/{number}]`, with log files in a
//! data folder on the disk.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::Path;

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, installed_settings, seed, state_with};
use serde_json::{Value, json};

/// The application with a data folder.
fn app_in(database: &Database, data: &Path) -> Router {
    let state = state_with(database, installed_settings(&[])).with_data_folder(data.to_owned());
    router(state).expect("the route table is valid")
}

/// A record as the server writes it.
fn record(level: &str, message: &str, trace_id: &str) -> String {
    json!({
        "log_level": level,
        "message": message,
        "channel": "Tests",
        "username": "System",
        "trace_id": trace_id,
        "file": "crates/api/tests/log.rs",
        "line": 1,
        "context": {"n": 1},
        "remote_ip_address": null,
        "datetime": "2026-11-20 03:15:00.000000",
        "timezone": "UTC",
    })
    .to_string()
}

/// Writes the log file of a day with records `(level, message)`, the oldest first. The
/// trace id of each record is `<date>-<place>`.
fn write_day(data_dir: &Path, date: &str, records: &[(&str, &str)]) {
    let folder = data_dir.join("logs");
    std::fs::create_dir_all(&folder).expect("the folder can be made");
    let text: String = records
        .iter()
        .enumerate()
        .map(|(place, (level, message))| record(level, message, &format!("{date}-{place}")) + "\n")
        .collect();
    std::fs::write(folder.join(format!("organizr-{date}.log")), text).expect("written");
}

/// Two days of records.
fn two_days(data: &Path) {
    write_day(
        data,
        "2026-11-19",
        &[("INFO", "old one"), ("ERROR", "old two")],
    );
    write_day(
        data,
        "2026-11-20",
        &[
            ("INFO", "new one"),
            ("WARNING", "new two"),
            ("DEBUG", "new three"),
            ("WARNING", "new four"),
        ],
    );
}

fn messages(data: &Value) -> Vec<String> {
    data["results"]
        .as_array()
        .expect("the results are a list")
        .iter()
        .map(|record| record["message"].as_str().expect("a message").to_owned())
        .collect()
}

#[tokio::test]
async fn only_admins_and_co_admins_read_and_purge_the_log() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        let app = app_in(database, folder.path());
        let super_user = seed::logged_in(database, &app, "sam", 2).await;
        let co_admin = seed::logged_in(database, &app, "pat", 1).await;
        let requests = [
            ("GET", "/log"),
            ("GET", "/log/0"),
            ("GET", "/log/all/2026-11-20-0"),
            ("DELETE", "/log"),
            ("DELETE", "/log/1"),
        ];
        for (method, path) in requests {
            for who in [As::Guest, As::User(&super_user)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
                assert_eq!(answer.message(), json!("Not Authorized"));
            }
        }
        assert_eq!(
            std::fs::read_dir(folder.path().join("logs"))
                .expect("listed")
                .count(),
            2
        );

        let answer = call(&app, As::User(&co_admin), "GET", "/log", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        let answer = call(&app, As::User(&co_admin), "DELETE", "/log/1", None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        // An application without a data folder has no log.
        let bare = installed_app_with(database, &[]);
        for (method, path) in requests {
            let answer = call(&bare, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{method} {path}");
            assert_eq!(answer.message(), json!("Logging not setup"));
        }
    })
    .await;
}

#[tokio::test]
async fn the_newest_file_is_read_newest_record_first_in_pages() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        let app = app_in(database, folder.path());

        let answer = call(&app, As::Admin, "GET", "/log", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.message(),
            json!("Results for log: organizr-2026-11-20.log")
        );
        let data = answer.data();
        assert_eq!(
            data["pageInfo"],
            json!({"results": 4, "totalPages": 1, "pageSize": 1000, "page": 1})
        );
        assert_eq!(
            messages(&data),
            ["new four", "new three", "new two", "new one"]
        );
        assert_eq!(data["files"], json!(["2026-11-20", "2026-11-19"]));
        // The fields of a record and of the page come in the order of PHP.
        assert!(
            answer.body.contains(
                "\"pageInfo\": {\n                \"results\": 4,\n                \"totalPages\": 1,\n                \"pageSize\": 1000,\n                \"page\": 1\n            },\n            \"results\": [\n                {\n                    \"log_level\": \"WARNING\",\n                    \"message\": \"new four\",\n                    \"channel\": \"Tests\",\n                    \"username\": \"System\",\n                    \"trace_id\": \"2026-11-20-3\","
            ),
            "{}",
            answer.body
        );

        // A page further on. The page size of the query comes back as text, as in PHP.
        let answer = call(&app, As::Admin, "GET", "/log/0?pageSize=3&offset=3", None).await;
        let data = answer.data();
        assert_eq!(
            data["pageInfo"],
            json!({"results": 4, "totalPages": 2, "pageSize": "3", "page": 2})
        );
        assert_eq!(messages(&data), ["new one"]);
        let answer = call(&app, As::Admin, "GET", "/log?pageSize=3&offset=4", None).await;
        assert_eq!(answer.data()["pageInfo"]["page"], json!(-1));
        assert_eq!(messages(&answer.data()), Vec::<String>::new());

        for query in ["pageSize=0", "pageSize=ten", "pageSize=-1", "offset=1.5", "offset="] {
            let answer = call(&app, As::Admin, "GET", &format!("/log?{query}"), None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{query}");
        }
        let answer = call(&app, As::Admin, "GET", "/log?offset=x", None).await;
        assert_eq!(answer.message(), json!("offset is not valid"));
    })
    .await;
}

#[tokio::test]
async fn the_filter_keeps_one_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        let app = app_in(database, folder.path());

        let answer = call(&app, As::Admin, "GET", "/log?filter=warning", None).await;
        let data = answer.data();
        assert_eq!(messages(&data), ["new four", "new two"]);
        assert_eq!(data["pageInfo"]["results"], json!(2));

        // An unknown filter is DEBUG, as in PHP. NONE keeps everything.
        let answer = call(&app, As::Admin, "GET", "/log?filter=loud", None).await;
        assert_eq!(messages(&answer.data()), ["new three"]);
        let answer = call(&app, As::Admin, "GET", "/log?filter=NONE", None).await;
        assert_eq!(answer.data()["pageInfo"]["results"], json!(4));
        let answer = call(&app, As::Admin, "GET", "/log/all?filter=ERROR", None).await;
        assert_eq!(messages(&answer.data()), ["old two"]);
    })
    .await;
}

#[tokio::test]
async fn the_number_picks_a_file_from_the_list_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        std::fs::write(folder.path().join("secret.txt"), "not a log").expect("written");
        let app = app_in(database, folder.path());

        let answer = call(&app, As::Admin, "GET", "/log/1", None).await;
        assert_eq!(
            answer.message(),
            json!("Results for log: organizr-2026-11-19.log")
        );
        assert_eq!(messages(&answer.data()), ["old two", "old one"]);

        // Every other value is the newest file, as in PHP.
        for number in [
            "0",
            "2",
            "01",
            "-1",
            "..%2F..%2Fsecret.txt",
            "organizr-2026-11-19.log",
        ] {
            let answer = call(&app, As::Admin, "GET", &format!("/log/{number}"), None).await;
            assert_eq!(answer.status, StatusCode::OK, "{number}");
            assert_eq!(
                answer.message(),
                json!("Results for log: organizr-2026-11-20.log"),
                "{number}"
            );
        }

        for number in ["all", "combined-logs"] {
            let answer = call(&app, As::Admin, "GET", &format!("/log/{number}"), None).await;
            assert_eq!(answer.message(), json!("Results for log: combined-logs"));
            assert_eq!(
                messages(&answer.data()),
                [
                    "new four",
                    "new three",
                    "new two",
                    "new one",
                    "old two",
                    "old one"
                ]
            );
        }
    })
    .await;
}

#[tokio::test]
async fn a_trace_id_gives_its_record() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        let app = app_in(database, folder.path());

        let answer = call(&app, As::Admin, "GET", "/log/all/2026-11-19-1", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(
            answer.message(),
            json!("Results for trace_id: 2026-11-19-1")
        );
        let data = answer.data();
        assert_eq!(data["message"], json!("old two"));
        assert_eq!(data["log_level"], json!("ERROR"));
        assert_eq!(data["context"], json!({"n": 1}));

        // The newest file alone does not hold it.
        let answer = call(&app, As::Admin, "GET", "/log/0/2026-11-19-1", None).await;
        assert_eq!(
            answer.data(),
            json!({
                "pageInfo": {"results": 0, "totalPages": 0, "pageSize": 1000, "page": -1},
                "results": [],
            })
        );

        // A level in the filter wins over the trace id, as in PHP.
        let answer = call(
            &app,
            As::Admin,
            "GET",
            "/log/all/2026-11-19-1?filter=INFO",
            None,
        )
        .await;
        assert_eq!(
            answer.message(),
            json!("Results for trace_id: 2026-11-19-1")
        );
        assert_eq!(messages(&answer.data()), ["new one", "old one"]);
    })
    .await;
}

#[tokio::test]
async fn a_log_file_is_purged() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        two_days(folder.path());
        let app = app_in(database, folder.path());
        let logs = folder.path().join("logs");

        for number in ["all", "combined-logs"] {
            let answer = call(&app, As::Admin, "DELETE", &format!("/log/{number}"), None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT);
            assert_eq!(
                answer.message(),
                json!("Cannot delete log [all] as it is not a real log")
            );
        }
        for number in ["2", "01", "x"] {
            let answer = call(&app, As::Admin, "DELETE", &format!("/log/{number}"), None).await;
            assert_eq!(answer.status, StatusCode::NOT_FOUND, "{number}");
            assert_eq!(answer.message(), json!("Log not found"));
        }

        let answer = call(&app, As::Admin, "DELETE", "/log/1", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.message(), json!("Log purged"));
        assert_eq!(answer.data(), Value::Null);
        assert!(!logs.join("organizr-2026-11-19.log").exists());
        assert!(logs.join("organizr-2026-11-20.log").exists());

        let answer = call(&app, As::Admin, "DELETE", "/log", None).await;
        assert_eq!(answer.message(), json!("Log purged"));
        assert!(!logs.join("organizr-2026-11-20.log").exists());

        // Without a log file.
        let answer = call(&app, As::Admin, "DELETE", "/log", None).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND);
        assert_eq!(answer.message(), json!("Log does not exist"));
        let answer = call(&app, As::Admin, "DELETE", "/log/0", None).await;
        assert_eq!(answer.message(), json!("Log not found"));
        let answer = call(&app, As::Admin, "GET", "/log", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.message(), json!("Results for log: "));
        assert_eq!(answer.data(), json!(false));
        let answer = call(&app, As::Admin, "GET", "/log/all", None).await;
        assert_eq!(answer.data()["pageInfo"]["results"], json!(0));
        assert_eq!(answer.data()["files"], json!([]));
    })
    .await;
}
