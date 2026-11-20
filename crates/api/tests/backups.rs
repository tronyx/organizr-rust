//! The backup routes, with a data folder on the disk.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use organizr_api::backup::{self, RestoreOptions};
use organizr_api::router;
use organizr_db::{Database, Engine};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, installed_settings, seed, state_with};
use serde_json::{Value, json};
use tower::ServiceExt;

/// The application with a data folder.
fn app_in(database: &Database, data: &Path, settings: &[(&str, Value)]) -> Router {
    let state =
        state_with(database, installed_settings(settings)).with_data_folder(data.to_owned());
    router(state).expect("the route table is valid")
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("the file is in a folder"))
        .expect("the folder can be made");
    std::fs::write(path, text).expect("the file can be written");
}

async fn list(app: &Router) -> Answer {
    call(app, As::Admin, "GET", "/backup", None).await
}

fn names(answer: &Answer) -> Vec<String> {
    answer.data()["files"]
        .as_array()
        .map_or_else(Vec::new, |files| {
            files
                .iter()
                .map(|file| file["name"].as_str().expect("a name is a text").to_owned())
                .collect()
        })
}

/// Downloads a file as the Admin: the status, two headers and the bytes.
async fn download(app: &Router, name: &str) -> (StatusCode, String, String, Vec<u8>) {
    let request = Request::builder()
        .uri(format!("/api/v2/backup/{name}"))
        .header("token", seed::API_KEY)
        .body(Body::empty())
        .expect("the request is valid");
    let response = app.clone().oneshot(request).await.expect("the app answers");
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    };
    let (status, kind, disposition) = (
        response.status(),
        header("content-type"),
        header("content-disposition"),
    );
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("the body can be read")
        .to_bytes()
        .to_vec();
    (status, kind, disposition, bytes)
}

#[tokio::test]
async fn backup_is_made_listed_downloaded_and_restored() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        write(&data.join("userTabs/logo.png"), "a picture");
        write(&data.join("secret.key"), "the key");
        write(&data.join("cache/tv-1.jpg"), "a cached picture");
        let app = app_in(database, &data, &[]);

        let empty = list(&app).await;
        assert_eq!(empty.status, StatusCode::OK, "{}", empty.body);
        assert_eq!(
            empty.data(),
            json!({"total_size": "0.00 B", "total_files": 0, "files": null})
        );

        let made = call(&app, As::Admin, "POST", "/backup", None).await;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        assert_eq!(made.message(), json!("Backup has been created"));
        let listed = list(&app).await;
        assert_eq!(listed.data()["total_files"], json!(1));
        let name = names(&listed).remove(0);
        assert!(backup::is_backup_name(&name), "{name}");
        assert!(
            name.starts_with("backup[20") && name.ends_with("].zip"),
            "{name}"
        );
        let row = &listed.data()["files"][0];
        assert_eq!(row["date"].as_str().map(str::len), Some(20));
        assert!(row["size"].as_str().is_some_and(|size| size.ends_with('B')));
        // Nothing of the work in between is left in the folder.
        let left: Vec<String> = std::fs::read_dir(data.join("backups"))
            .expect("the folder is there")
            .map(|entry| {
                entry
                    .expect("it can be read")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(left, std::slice::from_ref(&name));

        let (status, kind, disposition, bytes) = download(&app, &name.replace(' ', "%20")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(kind, "application/zip");
        assert_eq!(disposition, format!("attachment; filename=\"{name}\""));
        assert!(bytes.starts_with(b"PK"));

        // The download is a backup that the restore takes.
        let file = folder.path().join("downloaded.zip");
        std::fs::write(&file, &bytes).expect("the file can be written");
        let new = folder.path().join("new");
        let new_database = new.join("organizr.db");
        let options = RestoreOptions {
            data_dir: &new,
            database_file: Some(&new_database),
            overwrite: false,
        };
        let restored = backup::restore(&file, options).expect("the backup is restored");
        let is_sqlite = database.engine() == Engine::Sqlite;
        assert_eq!(restored.manifest.has_database, is_sqlite);
        assert_eq!(
            restored.manifest.engine,
            if is_sqlite { "sqlite" } else { "postgres" }
        );
        assert_eq!(restored.database, is_sqlite);
        assert_eq!(restored.files, 1);
        assert_eq!(
            std::fs::read_to_string(new.join("userTabs/logo.png"))
                .ok()
                .as_deref(),
            Some("a picture")
        );
        assert!(!new.join("secret.key").exists());
        assert!(!new.join("cache").exists());
        if is_sqlite {
            // The copy is a database with the records of the install.
            let url = format!(
                "sqlite:{}",
                new_database.display().to_string().replace('\\', "/")
            );
            let copy = Database::connect(&url).await.expect("the copy opens");
            let users = copy.users().count().await.expect("the copy has the tables");
            let here = database.users().count().await.expect("the count works");
            assert_eq!(users, here);
            copy.close().await.expect("the copy closes");
        }
    })
    .await;
}

#[tokio::test]
async fn old_backups_over_the_limit_are_deleted() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let app = app_in(
            database,
            folder.path(),
            &[("keepBackupsCountCron", json!("2"))],
        );
        for _ in 0..3 {
            let made = call(&app, As::Admin, "POST", "/backup", None).await;
            assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        }
        assert_eq!(names(&list(&app).await).len(), 2);

        // The number 0 keeps every backup.
        let app = app_in(
            database,
            folder.path(),
            &[("keepBackupsCountCron", json!("0"))],
        );
        call(&app, As::Admin, "POST", "/backup", None).await;
        assert_eq!(names(&list(&app).await).len(), 3);
    })
    .await;
}

#[tokio::test]
async fn backups_go_to_the_folder_of_the_settings_when_it_exists() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        let other = data.join("my-backups");
        std::fs::create_dir_all(&other).expect("the folder can be made");
        let location = [("backupLocation", json!(other.display().to_string()))];
        let app = app_in(database, &data, &location);
        for _ in 0..2 {
            call(&app, As::Admin, "POST", "/backup", None).await;
        }
        let all = backup::list(&other).expect("the folder can be read");
        assert_eq!(all.len(), 2);
        assert!(!data.join("backups").exists());
        // The second backup does not hold the first one.
        let new = folder.path().join("new");
        let options = RestoreOptions {
            data_dir: &new,
            database_file: None,
            overwrite: false,
        };
        let restored = backup::restore(&other.join(&all[0].name), options).expect("it is restored");
        assert_eq!(restored.files, 0);

        // A folder that does not exist is not used.
        let missing = [(
            "backupLocation",
            json!(folder.path().join("missing").display().to_string()),
        )];
        let app = app_in(database, &data, &missing);
        call(&app, As::Admin, "POST", "/backup", None).await;
        assert_eq!(
            backup::list(&data.join("backups")).expect("it works").len(),
            1
        );
    })
    .await;
}

#[tokio::test]
async fn only_a_backup_in_the_folder_is_sent_or_deleted() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path().join("data");
        write(&data.join("secret.key"), "the key");
        write(&data.join("backups/notes.txt"), "no backup");
        write(&data.join("backups/old.zip"), "PK an old backup");
        let app = app_in(database, &data, &[]);

        for name in [
            "..%2Fsecret.key",
            "notes.txt",
            "missing.zip",
            "..%2F..%2Fdata%2Fbackups%2Fold.zip",
        ] {
            let (status, ..) = download(&app, name).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{name}");
        }
        let (status, _, _, bytes) = download(&app, "old.zip").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(bytes, b"PK an old backup");

        let delete = |name: &'static str| {
            let path = format!("/backup/{name}");
            let app = app.clone();
            async move { call(&app, As::Admin, "DELETE", &path, None).await }
        };
        let refused = delete("notes.txt").await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("notes.txt is not approved to be deleted")
        );
        let refused = delete("..%2Fsecret.key").await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("secret.key is not approved to be deleted")
        );
        assert!(data.join("secret.key").exists());
        let missing = delete("missing.zip").await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);
        assert_eq!(missing.message(), json!("File does not exist"));
        let done = delete("old.zip").await;
        assert_eq!(done.status, StatusCode::OK, "{}", done.body);
        assert_eq!(done.message(), json!("old.zip has been deleted"));
        assert!(!data.join("backups/old.zip").exists());
        assert!(data.join("backups/notes.txt").exists());
    })
    .await;
}

#[tokio::test]
async fn only_an_admin_uses_the_backup_routes() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        write(&folder.path().join("backups/old.zip"), "PK an old backup");
        let app = app_in(database, folder.path(), &[]);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let admin = seed::logged_in(database, &app, "zed", 0).await;
        let requests = [
            ("GET", "/backup"),
            ("POST", "/backup"),
            ("GET", "/backup/old.zip"),
            ("DELETE", "/backup/old.zip"),
        ];
        for (method, path) in requests {
            for who in [As::Guest, As::User(&boss)] {
                let answer = call(&app, who, method, path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{method} {path}");
            }
        }
        assert!(folder.path().join("backups/old.zip").exists());
        let answer = call(&app, As::User(&admin), "GET", "/backup", None).await;
        assert_eq!(answer.status, StatusCode::OK);

        // An application without a data folder has no backups.
        let bare = installed_app_with(database, &[]);
        for (method, path) in requests {
            let answer = call(&bare, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{method} {path}");
            assert_eq!(answer.message(), json!("Backups are not set up"));
        }
    })
    .await;
}
