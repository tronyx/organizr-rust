//! The routes of the extensions, with two real extensions: the template of the
//! examples, and one that tries what an extension must not be able to do.
//!
//! The tests build both for WebAssembly, so they need the target `wasm32-wasip2`.

#![cfg(feature = "wasm-plugins")]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_settings, seed, state_with};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;

const CRATE: &str = env!("CARGO_MANIFEST_DIR");

/// Builds an extension and returns the bytes of its component.
fn build(folder: &Path, name: &str) -> Vec<u8> {
    let target = Path::new(CRATE).join("../../target/extension-guests");
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args([
            "build",
            "--release",
            "--target",
            "wasm32-wasip2",
            "--manifest-path",
        ])
        .arg(folder.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("CARGO_BUILD_TARGET")
        .output()
        .expect("cargo can be started");
    assert!(
        output.status.success(),
        "the extension {name} cannot be built: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file = target.join(format!("wasm32-wasip2/release/{name}.wasm"));
    std::fs::read(&file).unwrap_or_else(|error| panic!("cannot read {}: {error}", file.display()))
}

fn template() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let folder = PathBuf::from(CRATE).join("../../examples/extension-template");
        build(&folder, "extension_template")
    })
}

fn hostile() -> &'static [u8] {
    static BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    BYTES.get_or_init(|| {
        let folder = PathBuf::from(CRATE).join("../plugins/tests/guests/hostile");
        build(&folder, "hostile_extension")
    })
}

/// Puts an extension into the data folder.
fn put(data: &Path, id: &str, manifest: &Value, component: Option<&[u8]>) {
    let folder = data.join("plugins").join(id);
    std::fs::create_dir_all(&folder).expect("the folder can be made");
    std::fs::write(folder.join("plugin.json"), manifest.to_string()).expect("writable");
    if let Some(component) = component {
        std::fs::write(folder.join("plugin.wasm"), component).expect("writable");
    }
}

fn hello_manifest() -> Value {
    json!({
        "name": "Hello",
        "version": "1.0.0",
        "description": "Greets the caller.",
        "author": "Sam",
        "permissions": ["storage"],
        "least_access": 4,
    })
}

fn app_in(database: &Database, data: &Path) -> Router {
    let state = state_with(database, installed_settings(&[])).with_data_folder(data.to_owned());
    router(state).expect("the route table is valid")
}

async fn rows(app: &Router) -> Vec<Value> {
    let answer = call(app, As::Admin, "GET", "/extensions", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .data()
        .as_array()
        .expect("the data is a list")
        .clone()
}

/// Approves an extension with what the list shows for it.
async fn approve(app: &Router, id: &str) -> Answer {
    let all = rows(app).await;
    let row = all
        .iter()
        .find(|row| row["id"] == id)
        .expect("the extension is listed");
    let body = json!({
        "sha256": row["sha256"],
        "permissions": row["permissions"],
        "least_access": row["least_access"],
    });
    call(
        app,
        As::Admin,
        "POST",
        &format!("/extensions/{id}/enable"),
        Some(body),
    )
    .await
}

/// Sends a request and gives the status, the headers and the body as text.
async fn raw(
    app: &Router,
    cookie: Option<&str>,
    method: &str,
    path: &str,
    body: &str,
) -> (StatusCode, axum::http::HeaderMap, String) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/api/v2{path}"))
        .header("content-type", "text/plain")
        .header("x-secret", "not for the extension");
    request = match cookie {
        Some(cookie) => request.header("cookie", cookie),
        None => request.header("token", seed::API_KEY),
    };
    let request = request
        .body(Body::from(body.to_owned()))
        .expect("the request is valid");
    let response = app.clone().oneshot(request).await.expect("the app answers");
    let (status, headers) = (response.status(), response.headers().clone());
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("readable")
        .to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

#[tokio::test]
async fn extension_answers_only_after_an_admin_approved_it() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path();
        put(data, "hello", &hello_manifest(), Some(template()));
        put(
            data,
            "broken",
            &json!({"name": "Broken", "version": "1"}),
            None,
        );
        put(
            data,
            "greedy",
            &json!({"name": "Greedy", "version": "1", "permissions": ["network"]}),
            Some(template()),
        );
        put(data, "Not An Id", &hello_manifest(), Some(template()));
        let app = app_in(database, data);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        let all = rows(&app).await;
        let ids: Vec<&str> = all.iter().filter_map(|row| row["id"].as_str()).collect();
        assert_eq!(ids, ["broken", "greedy", "hello"]);
        let hello = &all[2];
        assert_eq!(hello["name"], json!("Hello"));
        assert_eq!(hello["permissions"], json!(["storage"]));
        assert_eq!(hello["least_access"], json!(4));
        assert_eq!(hello["sha256"].as_str().map(str::len), Some(64));
        assert_eq!(
            (
                &hello["enabled"],
                &hello["approved"],
                &hello["changed"],
                &hello["problem"]
            ),
            (&json!(false), &json!(false), &json!(false), &Value::Null)
        );
        assert!(
            all[0]["problem"]
                .as_str()
                .is_some_and(|text| text.contains("plugin.wasm"))
        );
        assert!(
            all[1]["problem"]
                .as_str()
                .is_some_and(|text| text.contains("network"))
        );

        // Nothing answers before the approval, and only an Admin approves.
        let (status, ..) = raw(&app, None, "GET", "/extension/hello/hello", "").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let body =
            json!({"sha256": hello["sha256"], "permissions": ["storage"], "least_access": 4});
        let path = "/extensions/hello/enable";
        for who in [As::Guest, As::User(&robin), As::User(&boss)] {
            let refused = call(&app, who, "POST", path, Some(body.clone())).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        }
        // The approval is for the file and the permissions that the Admin saw.
        let other_file =
            json!({"sha256": "0".repeat(64), "permissions": ["storage"], "least_access": 4});
        let refused = call(&app, As::Admin, "POST", path, Some(other_file)).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert!(
            refused.body.contains("file of the extension changed"),
            "{}",
            refused.body
        );
        let fewer = json!({"sha256": hello["sha256"], "permissions": [], "least_access": 4});
        let refused = call(&app, As::Admin, "POST", path, Some(fewer)).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert!(
            refused
                .body
                .contains("permissions of the extension changed"),
            "{}",
            refused.body
        );
        // And for the level that the Admin saw, also when the request names none.
        for level in [json!(999), json!("1"), Value::Null] {
            let mut other_level = json!({"sha256": hello["sha256"], "permissions": ["storage"]});
            if !level.is_null() {
                other_level["least_access"] = level.clone();
            }
            let refused = call(&app, As::Admin, "POST", path, Some(other_level)).await;
            assert_eq!(refused.status, StatusCode::CONFLICT, "{level}");
            assert!(
                refused.body.contains("level of the extension changed"),
                "{}",
                refused.body
            );
        }
        for id in ["broken", "greedy"] {
            let refused = approve(&app, id).await;
            assert_eq!(refused.status, StatusCode::CONFLICT, "{id}");
        }
        let missing = call(
            &app,
            As::Admin,
            "POST",
            "/extensions/nothing/enable",
            Some(body),
        )
        .await;
        assert_eq!(missing.status, StatusCode::NOT_FOUND);

        let approved = approve(&app, "hello").await;
        assert_eq!(approved.status, StatusCode::OK, "{}", approved.body);
        assert_eq!(approved.message(), json!("Extension enabled"));
        let hello = &rows(&app).await[2];
        assert_eq!(
            (&hello["enabled"], &hello["approved"]),
            (&json!(true), &json!(true))
        );
        // A Co-Admin sees the list.
        let seen = call(&app, As::User(&boss), "GET", "/extensions", None).await;
        assert_eq!(seen.status, StatusCode::OK);
        let seen = call(&app, As::User(&robin), "GET", "/extensions", None).await;
        assert_eq!(seen.status, StatusCode::UNAUTHORIZED);

        // The extension answers a caller of its level, and knows who the caller is.
        let (status, headers, text) =
            raw(&app, Some(&robin), "GET", "/extension/hello/hello", "").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(text, "Hello, robin!");
        assert_eq!(headers["content-type"], "text/plain; charset=utf-8");
        assert_eq!(headers["x-content-type-options"], "nosniff");
        let policy = headers["content-security-policy"]
            .to_str()
            .expect("it is text");
        assert!(
            policy.starts_with("sandbox ") && !policy.contains("allow-same-origin"),
            "{policy}"
        );
        let guest = call(&app, As::Guest, "GET", "/extension/hello/hello", None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);

        let (_, _, text) = raw(
            &app,
            Some(&robin),
            "GET",
            "/extension/hello/echo?a=1&b=2",
            "some text",
        )
        .await;
        assert_eq!(text, "GET /echo?a=1&b=2 level=4 body=some text");
        let (status, _, text) = raw(&app, None, "GET", "/extension/hello", "").await;
        assert_eq!(
            (status, text.as_str()),
            (StatusCode::NOT_FOUND, "This extension has no such route.")
        );
        let (status, ..) = raw(
            &app,
            None,
            "POST",
            "/extension/hello/echo",
            &"a".repeat(1024 * 1024 + 1),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    })
    .await;
}

#[tokio::test]
async fn values_stay_and_a_changed_file_stops_the_extension() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path();
        put(data, "hello", &hello_manifest(), Some(template()));
        let app = app_in(database, data);
        assert_eq!(approve(&app, "hello").await.status, StatusCode::OK);

        for expected in ["1", "2"] {
            let (status, _, text) = raw(&app, None, "POST", "/extension/hello/count", "").await;
            assert_eq!((status, text.as_str()), (StatusCode::OK, expected));
        }
        // The values are in the database, so another server on it counts on.
        let other = app_in(database, data);
        let (_, _, text) = raw(&other, None, "POST", "/extension/hello/count", "").await;
        assert_eq!(text, "3");

        // Switched off, the extension is not there. Switched on again, it has its values.
        let off = call(&app, As::Admin, "POST", "/extensions/hello/disable", None).await;
        assert_eq!(off.status, StatusCode::OK);
        assert_eq!(off.message(), json!("Extension disabled"));
        let (status, ..) = raw(&app, None, "POST", "/extension/hello/count", "").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(approve(&app, "hello").await.status, StatusCode::OK);
        let (_, _, text) = raw(&app, None, "POST", "/extension/hello/count", "").await;
        assert_eq!(text, "4");

        // Another file under the same name does not run.
        std::fs::write(data.join("plugins/hello/plugin.wasm"), hostile()).expect("writable");
        let (status, _, text) = raw(&app, None, "GET", "/extension/hello/environment", "").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(text.contains("changed after it was approved"), "{text}");
        let row = &rows(&app).await[0];
        assert_eq!(
            (&row["enabled"], &row["approved"], &row["changed"]),
            (&json!(false), &json!(false), &json!(true))
        );

        // Approved, the hostile file runs, in its sandbox.
        assert_eq!(approve(&app, "hello").await.status, StatusCode::OK);
        let (status, _, text) = raw(&app, None, "GET", "/extension/hello/environment", "").await;
        assert_eq!(
            (status, text.as_str()),
            (StatusCode::OK, "0 variables, 0 arguments")
        );
        let (status, _, text) = raw(&app, None, "GET", "/extension/hello/spin", "").await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert!(text.contains("the extension stopped"), "{text}");
        let (status, ..) = raw(&app, None, "GET", "/extension/hello/big", "").await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        // The server answers as before.
        let (status, ..) = raw(&app, None, "GET", "/extension/hello/environment", "").await;
        assert_eq!(status, StatusCode::OK);

        // A delete takes the folder, the approval and the values.
        let robin = seed::logged_in(database, &app, "robin", 1).await;
        let refused = call(&app, As::User(&robin), "DELETE", "/extensions/hello", None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        let deleted = call(&app, As::Admin, "DELETE", "/extensions/hello", None).await;
        assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
        assert_eq!(deleted.message(), json!("Extension deleted"));
        assert!(!data.join("plugins/hello").exists());
        assert_eq!(rows(&app).await.len(), 0);
        assert_eq!(
            database.extensions().find("hello").await.expect("it loads"),
            None
        );
        for path in ["/extensions/hello", "/extensions/..%2F..%2Fdata"] {
            let again = call(&app, As::Admin, "DELETE", path, None).await;
            assert_eq!(again.status, StatusCode::NOT_FOUND, "{path}");
        }
        assert!(data.exists());
    })
    .await;
}

fn sha256_of(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// A stand-in for a marketplace: its list, and the files that the list names.
async fn marketplace() -> StandIn {
    // The address of the stand-in is known only after its start, so the list names
    // paths, and the route of the list writes the addresses.
    async fn list(headers: axum::http::HeaderMap) -> axum::Json<Value> {
        let host = headers
            .get("host")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let entry = |id: &str, file: &str, sha256: String, permissions: Value| {
            let download = match file.strip_prefix("local/") {
                // The same server by another name: a host on this computer that is not
                // the host of the list.
                Some(file) => format!(
                    "http://localhost:{}/files/{file}",
                    host.rsplit(':').next().unwrap_or_default()
                ),
                None => format!("http://{host}/files/{file}"),
            };
            json!({
                "id": id,
                "name": format!("The {id} extension"),
                "version": "1.2.0",
                "description": "From the marketplace.",
                "author": "Sam",
                "permissions": permissions,
                "least_access": 4,
                "download": download,
                "sha256": sha256,
            })
        };
        let good = sha256_of(template());
        axum::Json(json!({"extensions": [
            entry("hello", "hello.wasm", good.clone(), json!(["storage"])),
            // The file is not the file that the list names.
            entry("swapped", "hostile.wasm", good.clone(), json!([])),
            entry("greedy", "hello.wasm", good.clone(), json!(["network"])),
            entry("missing", "nothing.wasm", good.clone(), json!([])),
            entry("local", "local/hello.wasm", good.clone(), json!([])),
            // The file moved on the host of the list, as a release file of GitHub moves
            // to another public host.
            entry("moved", "moved.wasm", good.clone(), json!([])),
            // A redirect to another host on this computer.
            entry("bounced", "bounced.wasm", good.clone(), json!([])),
            entry("Bad Id", "hello.wasm", good, json!([])),
        ]}))
    }
    /// Sends the download to the same file on the same server by another name, which is
    /// a host on this computer that is not the host of the list.
    async fn bounced(headers: axum::http::HeaderMap) -> axum::response::Redirect {
        let host = headers
            .get("host")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        axum::response::Redirect::temporary(&format!(
            "http://localhost:{}/files/hello.wasm",
            host.rsplit(':').next().unwrap_or_default()
        ))
    }
    StandIn::start(
        Router::new()
            .route("/extensions.json", axum::routing::get(list))
            .route(
                "/files/hello.wasm",
                axum::routing::get(|| async { template() }),
            )
            .route(
                "/files/hostile.wasm",
                axum::routing::get(|| async { hostile() }),
            )
            .route(
                "/files/moved.wasm",
                axum::routing::get(|| async {
                    axum::response::Redirect::permanent("/files/hello.wasm")
                }),
            )
            .route("/files/bounced.wasm", axum::routing::get(bounced)),
    )
    .await
}

fn app_with_market(database: &Database, data: &Path, list: &str) -> Router {
    let state = state_with(database, installed_settings(&[]))
        .with_data_folder(data.to_owned())
        .with_extension_marketplace(list);
    router(state).expect("the route table is valid")
}

#[tokio::test]
async fn extension_of_the_marketplace_is_installed_when_its_file_is_the_named_one() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path();
        let market = marketplace().await;
        let app = app_with_market(database, data, &format!("{}/extensions.json", market.url()));
        let boss = seed::logged_in(database, &app, "pat", 1).await;

        let listed = call(
            &app,
            As::User(&boss),
            "GET",
            "/extensions/marketplace",
            None,
        )
        .await;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        let offered = listed.data();
        let ids: Vec<&str> = offered
            .as_array()
            .expect("the data is a list")
            .iter()
            .filter_map(|row| row["id"].as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "hello", "swapped", "greedy", "missing", "local", "moved", "bounced"
            ]
        );
        assert_eq!(offered[0]["installed"], json!(false));
        assert_eq!(offered[0]["problem"], Value::Null);
        assert!(
            offered[2]["problem"]
                .as_str()
                .is_some_and(|text| text.contains("network"))
        );

        // Only an Admin installs.
        let path = "/extensions/marketplace/hello";
        let refused = call(&app, As::User(&boss), "POST", path, None).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert!(!data.join("plugins").exists());

        let installed = call(&app, As::Admin, "POST", path, None).await;
        assert_eq!(installed.status, StatusCode::OK, "{}", installed.body);
        let file = data.join("plugins/hello/plugin.wasm");
        assert_eq!(std::fs::read(&file).expect("the file is there"), template());
        assert!(!data.join("plugins/hello/plugin.wasm.part").exists());
        let listed = call(&app, As::Admin, "GET", "/extensions/marketplace", None).await;
        assert_eq!(
            (
                &listed.data()[0]["installed"],
                &listed.data()[0]["up_to_date"]
            ),
            (&json!(true), &json!(true))
        );

        // The installed extension waits for the approval, with what the list named.
        let row = &rows(&app).await[0];
        assert_eq!(row["name"], json!("The hello extension"));
        assert_eq!(row["permissions"], json!(["storage"]));
        assert_eq!(row["least_access"], json!(4));
        assert_eq!(
            (&row["approved"], &row["enabled"]),
            (&json!(false), &json!(false))
        );
        let (status, ..) = raw(&app, None, "GET", "/extension/hello/hello", "").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(approve(&app, "hello").await.status, StatusCode::OK);
        let (status, _, text) = raw(&app, Some(&boss), "GET", "/extension/hello/hello", "").await;
        assert_eq!((status, text.as_str()), (StatusCode::OK, "Hello, pat!"));

        // A file on the disk that is not the file of the list shows as out of date.
        std::fs::write(&file, hostile()).expect("writable");
        let listed = call(&app, As::Admin, "GET", "/extensions/marketplace", None).await;
        assert_eq!(
            (
                &listed.data()[0]["installed"],
                &listed.data()[0]["up_to_date"]
            ),
            (&json!(true), &json!(false))
        );
        // A new install puts the file of the list back, which is the approved one.
        assert_eq!(
            call(&app, As::Admin, "POST", path, None).await.status,
            StatusCode::OK
        );
        let (status, ..) = raw(&app, Some(&boss), "GET", "/extension/hello/hello", "").await;
        assert_eq!(status, StatusCode::OK);

        // A redirect on the host of the list is followed.
        let moved = call(
            &app,
            As::Admin,
            "POST",
            "/extensions/marketplace/moved",
            None,
        )
        .await;
        assert_eq!(moved.status, StatusCode::OK, "{}", moved.body);
        assert_eq!(
            std::fs::read(data.join("plugins/moved/plugin.wasm")).expect("the file is there"),
            template()
        );
    })
    .await;
}

#[tokio::test]
async fn nothing_is_installed_when_the_file_or_the_entry_is_wrong() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let data = folder.path();
        let market = marketplace().await;
        let app = app_with_market(database, data, &format!("{}/extensions.json", market.url()));
        let install = |id: &'static str| {
            let app = app.clone();
            async move {
                let path = format!("/extensions/marketplace/{id}");
                call(&app, As::Admin, "POST", &path, None).await
            }
        };

        let swapped = install("swapped").await;
        assert_eq!(swapped.status, StatusCode::CONFLICT);
        assert!(
            swapped.body.contains("Nothing was installed"),
            "{}",
            swapped.body
        );
        let greedy = install("greedy").await;
        assert_eq!(greedy.status, StatusCode::CONFLICT);
        assert!(greedy.body.contains("network"), "{}", greedy.body);
        // A download from another host on this computer is refused.
        let local = install("local").await;
        assert_eq!(local.status, StatusCode::CONFLICT);
        assert_eq!(
            local.message(),
            json!("The extension could not be downloaded")
        );
        // So is a redirect to such a host, which is checked as the first address is.
        let bounced = install("bounced").await;
        assert_eq!(bounced.status, StatusCode::CONFLICT, "{}", bounced.body);
        assert_eq!(
            bounced.message(),
            json!("The extension could not be downloaded")
        );
        let missing = install("missing").await;
        assert_eq!(missing.status, StatusCode::CONFLICT);
        assert_eq!(
            missing.message(),
            json!("The extension could not be downloaded")
        );
        for id in ["nothing", "Bad%20Id", "marketplace"] {
            let path = format!("/extensions/marketplace/{id}");
            let unknown = call(&app, As::Admin, "POST", &path, None).await;
            assert_eq!(unknown.status, StatusCode::NOT_FOUND, "{id}");
        }
        assert!(!data.join("plugins").exists());

        // Without a marketplace, and with one that does not answer, there is no list.
        let none = app_in(database, data);
        for (method, path) in [
            ("GET", "/extensions/marketplace"),
            ("POST", "/extensions/marketplace/hello"),
        ] {
            let answer = call(&none, As::Admin, method, path, None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{method}");
            assert_eq!(answer.message(), json!("No marketplace is set up"));
        }
        for list in [
            "http://127.0.0.1:9/extensions.json",
            &format!("{}/files/hello.wasm", market.url()),
        ] {
            let down = app_with_market(database, data, list);
            let answer = call(&down, As::Admin, "GET", "/extensions/marketplace", None).await;
            assert_eq!(answer.status, StatusCode::CONFLICT, "{list}");
            assert_eq!(answer.message(), json!("Could not access the marketplace"));
            assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
        }
    })
    .await;
}
