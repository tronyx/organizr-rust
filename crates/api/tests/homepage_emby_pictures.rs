//! The picture cache of the Emby and Jellyfin items: the name of a cached picture comes
//! from the picture that was fetched, so a caller cannot put one picture under the name
//! of another (D-153).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use axum::http::{StatusCode, Uri, header};
use axum::response::IntoResponse;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_in, seed};
use serde_json::json;

/// A media server whose pictures are a JPEG header and the path that was asked for, so
/// the bytes of a picture tell which picture it is.
async fn media_server() -> StandIn {
    let routes = Router::new().fallback(|uri: Uri| async move {
        let mut body = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F'];
        body.extend_from_slice(format!("#{}", uri.path()).as_bytes());
        ([(header::CONTENT_TYPE, "image/jpeg")], body).into_response()
    });
    StandIn::start(routes).await
}

/// The path of the picture that an answer holds.
fn picture_of(body: &str) -> String {
    body.split_once('#')
        .map_or_else(String::new, |(_, path)| path.to_owned())
}

#[tokio::test]
async fn caller_cannot_put_one_picture_under_the_name_of_another() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let media = media_server().await;
        let folder = tempfile::tempdir().expect("a folder for the pictures");
        let app = installed_app_in(database, folder.path());
        let saved = call(
            &app,
            As::Admin,
            "PUT",
            "/config",
            Some(json!({
                "homepageJellyfinEnabled": true,
                "homepageJellyfinAuth": 4,
                "jellyfinURL": media.url(),
                "jellyfinToken": "made-up-media-server-key",
            })),
        )
        .await;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        let user = seed::logged_in(database, &app, "uma", 4).await;

        let own = "/homepage/image?source=jellyfin&type=Primary&img=815&height=600&width=400&key=815-list";
        let first = call(&app, As::Admin, "GET", own, None).await;
        assert_eq!(first.status, StatusCode::OK, "{}", first.body);
        assert_eq!(picture_of(&first.body), "/Items/815/Images/Primary");

        // A user asks for another picture under the key of the first one, and for a
        // fresh copy.
        let forged = "/homepage/image?source=jellyfin&type=Thumb&img=777&height=600&width=400&key=815-list$z";
        let answer = call(&app, As::User(&user), "GET", forged, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(picture_of(&answer.body), "/Items/777/Images/Thumb");

        // The first picture is still the one in the cache.
        let again = call(&app, As::Admin, "GET", own, None).await;
        assert_eq!(picture_of(&again.body), "/Items/815/Images/Primary");
        // A key with another ending is no name of the cache.
        let odd = "/homepage/image?source=jellyfin&type=Primary&img=815&height=600&width=400&key=815-x";
        let refused = call(&app, As::User(&user), "GET", odd, None).await;
        assert_eq!(refused.status, StatusCode::NOT_FOUND, "{}", refused.body);
    })
    .await;
}
