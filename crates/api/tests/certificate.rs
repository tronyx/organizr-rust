//! `POST /certificate/custom`: the upload of the certificate of an authority, and the
//! requests to a service that then trust it (D-98).

#![allow(
    clippy::expect_used,
    reason = "a test reads the fields of answers it knows, and its helpers may panic"
)]

use std::path::Path;

use axum::Router;
use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{ARR_KEY, sonarr_routes};
use organizr_testkit::http::{Answer, As, call, request, send};
use organizr_testkit::services::StandIn;
use organizr_testkit::tls::{AUTHORITY, CERTIFICATE_REQUEST, LOCALHOST_CERTIFICATE, LOCALHOST_KEY};
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};

const BOUNDARY: &str = "organizr-test-boundary";

/// The application with a data folder, and other values for some settings.
fn app_in(database: &Database, folder: &Path, settings: &[(&str, Value)]) -> Router {
    let state =
        state_with(database, installed_settings(settings)).with_data_folder(folder.to_owned());
    organizr_api::router(state).expect("the route table is valid")
}

/// Uploads a file in the field `file`, with the headers of the caller.
async fn upload(app: &Router, headers: &[(&str, &str)], name: &str, content: &[u8]) -> Answer {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\nContent-Type: application/x-pem-file\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    let content_type = format!("multipart/form-data; boundary={BOUNDARY}");
    let mut all = vec![("content-type", content_type.as_str())];
    all.extend_from_slice(headers);
    let mut built = request("POST", "/api/v2/certificate/custom", &all, "");
    *built.body_mut() = axum::body::Body::from(body);
    send(app, built).await
}

/// The headers of a script with the API key, which counts as the Admin.
const ADMIN: [(&str, &str); 1] = [("token", seed::API_KEY)];

fn stored(folder: &Path) -> Option<String> {
    std::fs::read_to_string(folder.join("cert").join("custom.pem")).ok()
}

#[tokio::test]
async fn only_the_admin_uploads_a_certificate() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let app = app_in(database, folder.path(), &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        for who in [
            vec![],
            vec![("cookie", user.as_str())],
            vec![("cookie", co_admin.as_str())],
        ] {
            let answer = upload(&app, &who, "ca.pem", AUTHORITY.as_bytes()).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{who:?}");
            assert_eq!(answer.message(), json!("Not Authorized"));
        }
        assert_eq!(stored(folder.path()), None);

        // A request from another site is refused, also for the Admin.
        let admin = seed::logged_in(database, &app, "ada", 0).await;
        let answer = upload(
            &app,
            &[("cookie", &admin), ("sec-fetch-site", "cross-site")],
            "ca.pem",
            AUTHORITY.as_bytes(),
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(stored(folder.path()), None);

        let answer = upload(&app, &[("cookie", &admin)], "ca.pem", AUTHORITY.as_bytes()).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("ca.pem has been uploaded"));
        assert_eq!(answer.data(), Value::Null);
        let kept = stored(folder.path()).expect("the certificate is stored");
        assert!(kept.starts_with("-----BEGIN CERTIFICATE-----\n"), "{kept}");
    })
    .await;
}

#[tokio::test]
async fn only_the_certificate_of_an_authority_is_taken() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let app = app_in(database, folder.path(), &[]);
        let leaf_and_key = format!("{LOCALHOST_CERTIFICATE}{LOCALHOST_KEY}");
        let too_large = "a".repeat(64 * 1024 + 1);
        let cases: [(&str, &[u8], StatusCode, &str); 7] = [
            (
                "ca.crt",
                AUTHORITY.as_bytes(),
                StatusCode::FORBIDDEN,
                "ca.crt is not approved to be uploaded",
            ),
            (
                "localhost.pem",
                LOCALHOST_CERTIFICATE.as_bytes(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "A certificate of the file is not that of a certificate authority",
            ),
            (
                "key.pem",
                LOCALHOST_KEY.as_bytes(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "The file holds a private key. Upload only the certificate of the authority",
            ),
            (
                "both.pem",
                leaf_and_key.as_bytes(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "The file holds a private key. Upload only the certificate of the authority",
            ),
            (
                "request.pem",
                CERTIFICATE_REQUEST.as_bytes(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "The file holds a part that is no certificate",
            ),
            (
                "garbage.pem",
                b"\x00\x01 this is no certificate",
                StatusCode::UNPROCESSABLE_ENTITY,
                "The file is no PEM file with a certificate",
            ),
            (
                "large.pem",
                too_large.as_bytes(),
                StatusCode::PAYLOAD_TOO_LARGE,
                "The certificate file has 65536 bytes at most",
            ),
        ];
        for (name, content, status, message) in cases {
            let answer = upload(&app, &ADMIN, name, content).await;
            assert_eq!(answer.status, status, "{name}: {}", answer.body);
            assert_eq!(answer.message(), json!(message), "{name}");
            assert_eq!(stored(folder.path()), None, "{name}");
        }
        // A key is never stored, and the answer never repeats the file.
        let answer = upload(&app, &ADMIN, "key.pem", LOCALHOST_KEY.as_bytes()).await;
        assert!(!answer.body.contains("PRIVATE KEY"), "{}", answer.body);

        // Without a file, PHP names an empty file.
        let mut empty = request(
            "POST",
            "/api/v2/certificate/custom",
            &[
                ("token", seed::API_KEY),
                ("content-type", "multipart/form-data; boundary=x"),
            ],
            "",
        );
        *empty.body_mut() = axum::body::Body::from("--x--\r\n");
        let answer = send(&app, empty).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        assert_eq!(answer.message(), json!(" is not approved to be uploaded"));
    })
    .await;
}

#[tokio::test]
async fn service_that_asks_for_the_certificate_trusts_the_authority() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let sonarr = StandIn::start_tls(sonarr_routes()).await;
        let settings = |custom: bool| {
            vec![
                ("sonarrURL", json!(sonarr.https_url())),
                ("sonarrToken", json!(ARR_KEY)),
                ("sonarrUseCustomCertificate", json!(custom)),
            ]
        };
        let test = async |app: &Router| call(app, As::Admin, "POST", "/test/sonarr", None).await;

        // No system trusts the authority of the tests.
        let app = app_in(database, folder.path(), &settings(true));
        assert_eq!(test(&app).await.status, StatusCode::INTERNAL_SERVER_ERROR);

        let answer = upload(&app, &ADMIN, "authority.pem", AUTHORITY.as_bytes()).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let answer = test(&app).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        // A service whose setting is off does not trust the authority.
        let app = app_in(database, folder.path(), &settings(false));
        assert_eq!(test(&app).await.status, StatusCode::INTERNAL_SERVER_ERROR);

        // A server that starts again reads the certificate from the data folder.
        let app = app_in(database, folder.path(), &settings(true));
        let answer = test(&app).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    })
    .await;
}
