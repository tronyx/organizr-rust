//! The `/image` routes through the real route table: what the recorded PHP answers
//! cannot show.
//!
//! The recording covers what both applications answer alike. These tests cover callers
//! of other levels, the checks of an uploaded file, and the address that sends an
//! uploaded picture (D-10, D-54).

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
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call, send};
use organizr_testkit::{installed_app, installed_app_in, seed};
use serde_json::{Value, json};

const BOUNDARY: &str = "test-boundary-0123456789";

/// The start of a PNG file, and a little more.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR-and-so-on";

/// A whole GIF file. All its bytes are plain text characters, so an answer that holds it
/// can be read as text.
const GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\x00\x00\x00;";

/// How many pictures come with the application.
const BUNDLED: usize = 168;

/// Uploads one file in the field `file`.
async fn upload(
    app: &Router,
    who: As<'_>,
    name: &str,
    content_type: &str,
    content: &[u8],
) -> Answer {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\
         Content-Type: {content_type}\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(content);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    let mut request = Request::builder()
        .method("POST")
        .uri("/api/v2/image")
        .header("host", "organizr.test")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
    request = match who {
        As::Guest => request,
        As::Admin => request.header("token", seed::API_KEY),
        As::User(cookie) => request.header("cookie", cookie),
    };
    let request = request
        .body(Body::from(body))
        .expect("the request is valid");
    send(app, request).await
}

/// The paths of all pictures as `GET /image` lists them.
async fn pictures(app: &Router) -> Vec<String> {
    let answer = call(app, As::Admin, "GET", "/image", None).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    answer
        .data()
        .as_array()
        .expect("the data is a list")
        .iter()
        .map(|path| path.as_str().expect("a path is text").to_owned())
        .collect()
}

/// Asks for a file at an address that is not under the API.
async fn get(app: &Router, path: &str) -> Answer {
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header("host", "organizr.test")
        .body(Body::empty())
        .expect("the request is valid");
    send(app, request).await
}

fn assert_refused(answer: &Answer, status: StatusCode, message: &str) {
    assert_eq!(answer.status, status, "{}", answer.body);
    assert_eq!(answer.message(), json!(message));
}

/// The names of the files in a folder, sorted.
fn files_in(folder: &Path) -> Vec<String> {
    // A folder that does not exist holds no files.
    let mut names: Vec<String> = std::fs::read_dir(folder).map_or_else(
        |_| Vec::new(),
        |entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        },
    );
    names.sort();
    names
}

#[tokio::test]
async fn pictures_are_managed_by_admins_and_co_admins() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let super_user = seed::logged_in(database, &app, "sam", 2).await;

        let refused = upload(&app, As::User(&super_user), "logo.png", "image/png", PNG).await;
        assert_refused(&refused, StatusCode::UNAUTHORIZED, "Not Authorized");
        for (method, path) in [
            ("GET", "/image"),
            ("GET", "/image/select"),
            ("DELETE", "/image/logo.png"),
        ] {
            let answer = call(&app, As::User(&super_user), method, path, None).await;
            assert_refused(&answer, StatusCode::UNAUTHORIZED, "Not Authorized");
        }
        assert_eq!(files_in(folder.path()), Vec::<String>::new());

        let stored = upload(&app, As::User(&co_admin), "logo.png", "image/png", PNG).await;
        assert_eq!(stored.status, StatusCode::OK, "{}", stored.body);
        assert_eq!(stored.message(), json!("logo.png has been uploaded"));
        let listed = call(&app, As::User(&co_admin), "GET", "/image", None).await;
        assert_eq!(listed.data().as_array().map(Vec::len), Some(BUNDLED + 1));
        let deleted = call(&app, As::User(&co_admin), "DELETE", "/image/logo.png", None).await;
        assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
        assert_eq!(files_in(folder.path()), Vec::<String>::new());
    })
    .await;
}

#[tokio::test]
async fn upload_must_be_a_picture_by_content_ending_and_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        let svg = b"<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";

        let refused: [(&str, &str, &[u8], &str); 9] = [
            // The browser says that it is a picture, and the content is none.
            (
                "logo.png",
                "image/png",
                b"<html>not a picture</html>",
                "File Type not approved",
            ),
            ("logo.svg", "image/svg+xml", svg, "File Type not approved"),
            ("logo.png", "image/png", b"", "File Type not approved"),
            // A picture with an ending that a browser shows as something else.
            (
                "logo.png.html",
                "image/png",
                PNG,
                "File Extension not approved",
            ),
            ("logo.svg", "image/png", PNG, "File Extension not approved"),
            ("logo", "image/png", PNG, "File Extension not approved"),
            // A name that a file system reads as more than a name.
            ("con:1.png", "image/png", PNG, "File Name not approved"),
            (".png", "image/png", PNG, "File Name not approved"),
            ("what?.png", "image/png", PNG, "File Name not approved"),
        ];
        for (name, content_type, content, message) in refused {
            let answer = upload(&app, As::Admin, name, content_type, content).await;
            assert_refused(&answer, StatusCode::FORBIDDEN, message);
            // PHP names the path of its temporary file here.
            assert_eq!(answer.data(), Value::Null, "{name}");
        }
        let text = upload(&app, As::Admin, "logo.png", "text/html", PNG).await;
        assert_refused(&text, StatusCode::FORBIDDEN, "File Type not image");
        assert_eq!(text.data(), json!("text/html"));

        assert_eq!(files_in(folder.path()), Vec::<String>::new());
        assert_eq!(pictures(&app).await.len(), BUNDLED);
    })
    .await;
}

#[tokio::test]
async fn upload_is_stored_in_the_folder_under_its_own_name_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let parent = tempfile::tempdir().expect("a folder can be made");
        let folder = parent.path().join("userTabs");
        let app = installed_app_in(database, &folder);

        // A browser can send a whole path. Only its last part is the name.
        for name in [
            "../escaped.png",
            "C:\\Users\\me\\My Logo (2).PNG",
            "  spaced.jpg  ",
        ] {
            let answer = upload(&app, As::Admin, name, "image/png", PNG).await;
            assert_eq!(answer.status, StatusCode::OK, "{name}: {}", answer.body);
        }

        assert_eq!(
            files_in(&folder),
            ["My Logo (2).PNG", "escaped.png", "spaced.jpg"]
        );
        assert_eq!(files_in(parent.path()), ["userTabs"]);
        let content = std::fs::read(folder.join("escaped.png")).expect("the file exists");
        assert_eq!(content, PNG);
    })
    .await;
}

#[tokio::test]
async fn upload_has_a_size_limit() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        let limit = 10 * 1024 * 1024;
        let mut large = PNG.to_vec();
        large.resize(limit, 0);

        let at_the_limit = upload(&app, As::Admin, "large.png", "image/png", &large).await;
        assert_eq!(at_the_limit.status, StatusCode::OK, "{}", at_the_limit.body);

        large.resize(limit + 128 * 1024, 0);
        let too_large = upload(&app, As::Admin, "larger.png", "image/png", &large).await;
        assert_refused(
            &too_large,
            StatusCode::PAYLOAD_TOO_LARGE,
            "File is larger than 10 MB",
        );
        assert_eq!(files_in(folder.path()), ["large.png"]);
    })
    .await;
}

#[tokio::test]
async fn uploaded_picture_is_sent_to_everybody_as_a_picture_and_nothing_else() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let parent = tempfile::tempdir().expect("a folder can be made");
        let folder = parent.path().join("userTabs");
        let app = installed_app_in(database, &folder);
        upload(&app, As::Admin, "My Logo.gif", "image/gif", GIF).await;
        // Files that the application did not store as pictures.
        std::fs::write(folder.join("note.txt"), "a note").expect("the file can be written");
        std::fs::write(parent.path().join("secret.key"), "a key").expect("the file can be written");
        std::fs::write(parent.path().join("other.png"), PNG).expect("the file can be written");

        let sent = get(&app, "/data/userTabs/My%20Logo.gif").await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.body.as_bytes(), GIF);
        let header = |name: &str| {
            sent.headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        assert_eq!(header("content-type").as_deref(), Some("image/gif"));
        assert_eq!(header("x-content-type-options").as_deref(), Some("nosniff"));
        assert_eq!(
            header("content-security-policy").as_deref(),
            Some("default-src 'none'; sandbox")
        );

        for path in [
            "/data/userTabs/no-such-picture.png",
            "/data/userTabs/note.txt",
            // Ways out of the folder.
            "/data/userTabs/..%2Fsecret.key",
            "/data/userTabs/..%2Fother.png",
            "/data/userTabs/%2E%2E%2Fother.png",
            "/data/userTabs/..%5Cother.png",
            "/data/userTabs/sub/My%20Logo.gif",
            "/data/userTabs/",
            "/data/other.png",
        ] {
            let answer = get(&app, path).await;
            assert_eq!(
                answer.status,
                StatusCode::NOT_FOUND,
                "{path}: {}",
                answer.body
            );
            assert!(!answer.body.contains("a key"), "{path}");
        }
    })
    .await;
}

#[tokio::test]
async fn uploaded_picture_stands_in_for_a_picture_of_the_application_with_its_name() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        let before = pictures(&app).await;
        assert!(before.contains(&"plugins/images/tabs/plex.png".to_owned()));

        upload(&app, As::Admin, "plex.png", "image/png", PNG).await;
        upload(&app, As::Admin, "Zebra.png", "image/png", PNG).await;
        upload(&app, As::Admin, "aardvark.png", "image/png", PNG).await;

        let listed = pictures(&app).await;
        assert_eq!(listed.len(), BUNDLED + 2);
        assert!(listed.contains(&"data/userTabs/plex.png".to_owned()));
        assert!(!listed.contains(&"plugins/images/tabs/plex.png".to_owned()));
        // The list follows the names of the files, without regard to letter case.
        assert_eq!(
            listed.first().map(String::as_str),
            Some("data/userTabs/aardvark.png")
        );
        assert_eq!(
            listed.last().map(String::as_str),
            Some("plugins/images/tabs/zwavejs.png")
        );
        let zebra = listed
            .iter()
            .position(|path| path == "data/userTabs/Zebra.png")
            .expect("the picture is listed");
        assert_eq!(zebra, listed.len() - 4, "{:?}", &listed[listed.len() - 4..]);

        // When the uploaded picture goes, the picture of the application is listed again.
        let deleted = call(&app, As::Admin, "DELETE", "/image/plex.png", None).await;
        assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
        assert!(
            pictures(&app)
                .await
                .contains(&"plugins/images/tabs/plex.png".to_owned())
        );
    })
    .await;
}

#[tokio::test]
async fn delete_takes_a_name_of_the_upload_folder_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let parent = tempfile::tempdir().expect("a folder can be made");
        let folder = parent.path().join("userTabs");
        let app = installed_app_in(database, &folder);
        upload(&app, As::Admin, "logo.png", "image/png", PNG).await;
        std::fs::write(parent.path().join("other.png"), PNG).expect("the file can be written");
        std::fs::write(parent.path().join("secret.key"), "a key").expect("the file can be written");

        let nothing = call(&app, As::Admin, "DELETE", "/image/0", None).await;
        assert_refused(
            &nothing,
            StatusCode::UNPROCESSABLE_ENTITY,
            "No image supplied",
        );
        let key = call(&app, As::Admin, "DELETE", "/image/..%2Fsecret.key", None).await;
        assert_refused(
            &key,
            StatusCode::CONFLICT,
            "data/userTabs/secret.key is not approved to be deleted",
        );
        // Only the last part of a path counts, and that file is not in the folder.
        let outside = call(&app, As::Admin, "DELETE", "/image/..%2Fother.png", None).await;
        assert_refused(
            &outside,
            StatusCode::NOT_FOUND,
            "data/userTabs/other.png does not exist",
        );

        assert_eq!(
            files_in(parent.path()),
            ["other.png", "secret.key", "userTabs"]
        );
        assert_eq!(files_in(&folder), ["logo.png"]);
    })
    .await;
}

#[tokio::test]
async fn chooser_list_refuses_a_page_or_a_length_that_is_no_count() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        for (query, name) in [
            ("page=0", "page"),
            ("page=-1", "page"),
            ("page=two", "page"),
            ("page=", "page"),
            ("limit=0", "limit"),
            ("limit=many", "limit"),
            ("limit=1.5", "limit"),
        ] {
            let answer = call(
                &app,
                As::Admin,
                "GET",
                &format!("/image/select?{query}"),
                None,
            )
            .await;
            assert_refused(
                &answer,
                StatusCode::UNPROCESSABLE_ENTITY,
                &format!("{name} is not valid"),
            );
        }
        // The search finds a name in any letter case.
        let found = call(&app, As::Admin, "GET", "/image/select?search=adguard", None).await;
        assert_eq!(found.data()["total"], json!(1));
        assert_eq!(
            found.data()["results"][0]["id"],
            json!("plugins/images/tabs/AdGuardHome.png")
        );
        // PHP takes the empty text and the text 0 for "no search".
        for query in ["search=", "search=0"] {
            let answer = call(
                &app,
                As::Admin,
                "GET",
                &format!("/image/select?{query}"),
                None,
            )
            .await;
            assert_eq!(answer.data()["total"], json!(BUNDLED), "{query}");
        }
        let last = call(
            &app,
            As::Admin,
            "GET",
            "/image/select?page=9&limit=20",
            None,
        )
        .await;
        assert_eq!(last.data()["results"].as_array().map(Vec::len), Some(8));
        assert_eq!(last.data()["pagination"]["more"], json!(false));
        let before_last = call(
            &app,
            As::Admin,
            "GET",
            "/image/select?page=8&limit=20",
            None,
        )
        .await;
        assert_eq!(before_last.data()["pagination"]["more"], json!(true));
    })
    .await;
}

#[tokio::test]
async fn application_without_a_picture_folder_lists_its_own_pictures_and_stores_none() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app(database);

        assert_eq!(pictures(&app).await.len(), BUNDLED);
        let answer = upload(&app, As::Admin, "logo.png", "image/png", PNG).await;
        assert_refused(
            &answer,
            StatusCode::INTERNAL_SERVER_ERROR,
            "File upload error",
        );
        let sent = get(&app, "/data/userTabs/logo.png").await;
        assert_eq!(sent.status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn list_holds_only_what_the_application_sends_as_a_picture() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        upload(&app, As::Admin, "logo.png", "image/png", PNG).await;
        // What somebody else can leave in the folder.
        for name in ["index.html", ".DS_Store", "note.txt", ".hidden.png"] {
            std::fs::write(folder.path().join(name), "x").expect("the file can be written");
        }
        std::fs::create_dir(folder.path().join("folder.png")).expect("the folder can be made");

        let listed = pictures(&app).await;

        assert_eq!(listed.len(), BUNDLED + 1);
        let uploaded: Vec<&String> = listed
            .iter()
            .filter(|path| path.starts_with("data/userTabs/"))
            .collect();
        assert_eq!(uploaded, ["data/userTabs/logo.png"]);
    })
    .await;
}

#[tokio::test]
async fn drawing_that_an_import_brought_is_sent_and_runs_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let folder = tempfile::tempdir().expect("a folder can be made");
        let app = installed_app_in(database, folder.path());
        // Nobody can upload a drawing. The import of a PHP install copies the drawings
        // that a person had put into the picture folders of that install.
        let drawing = "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(1)</script></svg>";
        std::fs::create_dir_all(folder.path()).expect("the folder can be made");
        std::fs::write(folder.path().join("Drawing.SVG"), drawing)
            .expect("the file can be written");

        let sent = get(&app, "/data/userTabs/Drawing.SVG").await;
        assert_eq!(sent.status, StatusCode::OK);
        assert_eq!(sent.body, drawing);
        let header = |name: &str| {
            sent.headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        };
        assert_eq!(header("content-type").as_deref(), Some("image/svg+xml"));
        assert_eq!(header("x-content-type-options").as_deref(), Some("nosniff"));
        // No script of the drawing runs, and it loads nothing. Its own styles work.
        assert_eq!(
            header("content-security-policy").as_deref(),
            Some("default-src 'none'; style-src 'unsafe-inline'; sandbox")
        );

        let listed = pictures(&app).await;
        assert!(
            listed.contains(&"data/userTabs/Drawing.SVG".to_owned()),
            "{listed:?}"
        );

        let deleted = call(&app, As::Admin, "DELETE", "/image/Drawing.SVG", None).await;
        assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
        assert_eq!(files_in(folder.path()), [] as [String; 0]);
    })
    .await;
}
