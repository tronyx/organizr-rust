//! Emby and Jellyfin on the homepage, against stand-ins for the two servers.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, JPEG, MEDIA_SERVER_KEY};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::services::StandIn;
use organizr_testkit::{installed_app_in, installed_app_with, seed};
use serde_json::{Value, json};

/// One of the two servers: its names, and the keys of its settings.
#[derive(Debug, Clone, Copy)]
struct Kind {
    /// The name in paths.
    path: &'static str,
    /// The name in texts.
    name: &'static str,
    enabled: &'static str,
    auth: &'static str,
    streams: (&'static str, &'static str),
    recent: (&'static str, &'static str),
    url: &'static str,
    token: &'static str,
    link: &'static str,
    /// The header that carries the key, and its value.
    header: (&'static str, &'static str),
    order: (&'static str, &'static str),
}

const EMBY: Kind = Kind {
    path: "emby",
    name: "Emby",
    enabled: "homepageEmbyEnabled",
    auth: "homepageEmbyAuth",
    streams: ("homepageEmbyStreams", "homepageEmbyStreamsAuth"),
    recent: ("homepageEmbyRecent", "homepageEmbyRecentAuth"),
    url: "embyURL",
    token: "embyToken",
    link: "homepageEmbyLink",
    header: ("x-emby-token", MEDIA_SERVER_KEY),
    order: ("homepageOrderembynowplaying", "homepageOrderembyrecent"),
};

const JELLYFIN: Kind = Kind {
    path: "jellyfin",
    name: "Jellyfin",
    enabled: "homepageJellyfinEnabled",
    auth: "homepageJellyfinAuth",
    streams: ("homepageJellyfinStreams", "homepageJellyStreamsAuth"),
    recent: ("homepageJellyfinRecent", "homepageJellyfinRecentAuth"),
    url: "jellyfinURL",
    token: "jellyfinToken",
    link: "homepageJellyfinLink",
    header: (
        "authorization",
        "MediaBrowser Token=\"made-up-media-server-key\"",
    ),
    order: (
        "homepageOrderjellyfinnowplaying",
        "homepageOrderjellyfinrecent",
    ),
};

async fn stand_in(kind: Kind) -> StandIn {
    if kind.path == "emby" {
        homepage_services::emby().await
    } else {
        homepage_services::jellyfin().await
    }
}

/// The settings of a server that shows both items to users, with other values for
/// some.
fn settings(kind: Kind, url: &str, more: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut settings = vec![
        (kind.enabled, json!(true)),
        (kind.auth, json!("4")),
        (kind.streams.0, json!(true)),
        (kind.streams.1, json!("4")),
        (kind.recent.0, json!(true)),
        (kind.recent.1, json!("4")),
        (kind.url, json!(url)),
        (kind.token, json!(MEDIA_SERVER_KEY)),
        (
            kind.link,
            json!("https://media.example.com/item?id={id}&serverId={serverId}"),
        ),
    ];
    for (key, value) in more {
        settings.retain(|(known, _)| known != key);
        settings.push((key, value.clone()));
    }
    settings
}

fn titles(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("a list of items")
        .iter()
        .map(|item| item["title"].as_str().unwrap_or("?").to_owned())
        .collect()
}

async fn streams_show_who_plays_to_the_level(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let app = installed_app_with(
        database,
        &settings(
            kind,
            &server.url(),
            &[("homepageShowStreamNames", json!(true))],
        ),
    );
    let user = seed::logged_in(database, &app, &format!("zed{}", kind.path), 4).await;
    let path = format!("/homepage/{}/streams", kind.path);

    let all = call(&app, As::Admin, "GET", &path, None).await;
    assert_eq!(all.status, StatusCode::OK, "{}", all.body);
    let data = all.data();
    // The player that plays nothing and the television channel are left out.
    assert_eq!(titles(&data["content"]), ["The Show", "The Film"]);
    let stream = &data["content"][0];
    assert_eq!(stream["type"], "tv");
    assert_eq!(stream["nowPlayingTitle"], "The Show - The One");
    assert_eq!(stream["nowPlayingBottom"], "S2 · E3");
    assert_eq!(stream["user"], "robin");
    assert_eq!(stream["userAddress"], "192.0.2.50");
    assert_eq!(stream["state"], "pause");
    assert_eq!(stream["watched"], 25);
    assert_eq!(stream["transcoded"], 40);
    assert_eq!(stream["bandwidth"], 4000);
    assert_eq!(stream["sessionType"], "Transcoding");
    assert_eq!(stream["session"], "device-one");
    assert_eq!(stream["id"], "made-up-server-id");
    assert_eq!(stream["key"], "30-list");
    assert_eq!(stream["nowPlayingKey"], "30-np");
    assert_eq!(
        stream["address"],
        "https://media.example.com/item?id=301&serverId=made-up-server-id"
    );
    assert_eq!(
        stream["nowPlayingImageURL"],
        format!(
            "api/v2/homepage/image?source={}&type=Backdrop&img=30&height=1350&width=2400&key=30-np",
            kind.path
        )
    );
    assert_eq!(stream["userStream"]["videoDecision"], "Transcode");
    assert_eq!(stream["userStream"]["sourceVideoCodec"], "hevc");
    let film = &data["content"][1];
    assert_eq!(film["state"], "play");
    assert_eq!(film["elapsed"], 0);
    assert_eq!(film["watched"], 0);
    assert_eq!(film["transcoded"], 100);
    assert_eq!(film["sessionType"], "Direct Playing");
    assert_eq!(film["metadata"]["genres"], json!(["Drama"]));

    // The key goes in a header, and not in the address.
    let seen = &server.seen()[0];
    assert_eq!(
        seen.path,
        "/Sessions?Fields=Overview%2CPeople%2CGenres%2CCriticRating%2CStudios%2CTaglines"
    );
    assert_eq!(seen.header(kind.header.0), Some(kind.header.1));

    // The names and the addresses are for Admins and Co-Admins.
    let some = call(&app, As::User(&user), "GET", &path, None).await;
    assert_eq!(some.status, StatusCode::OK, "{}", some.body);
    assert_eq!(some.data()["content"][0]["user"], "");
    assert_eq!(some.data()["content"][0]["userAddress"], "x.x.x.x");

    let guest = call(&app, As::Guest, "GET", &path, None).await;
    assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        guest.message(),
        json!("User not approved to view this homepage item")
    );
    assert_eq!(
        call(&app, As::User(&user), "GET", "/homepage/items", None)
            .await
            .data(),
        json!([
            {"key": kind.order.0, "kind": format!("{}streams", kind.path), "id": ""},
            {"key": kind.order.1, "kind": format!("{}recent", kind.path), "id": ""},
        ])
    );
}

#[tokio::test]
async fn streams_show_who_plays_only_to_callers_with_the_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        streams_show_who_plays_to_the_level(database, EMBY).await;
        streams_show_who_plays_to_the_level(database, JELLYFIN).await;
    })
    .await;
}

async fn recent_items_are_those_of_the_user(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let app = installed_app_with(
        database,
        &settings(kind, &server.url(), &[("homepageRecentLimit", json!("5"))]),
    );
    let path = format!("/homepage/{}/recent", kind.path);
    let fields = "Fields=Overview%2CPeople%2CGenres%2CCriticRating%2CStudios%2CTaglines";
    // Only the request to Emby names the types.
    let types = if kind.path == "emby" {
        "&IncludeItemTypes=Series%2CEpisode%2CMusicAlbum%2CAudio%2CMovie%2CVideo"
    } else {
        ""
    };

    // No user of the server has the name of the caller: the administrator is taken.
    let recent = call(&app, As::Admin, "GET", &path, None).await;
    assert_eq!(recent.status, StatusCode::OK, "{}", recent.body);
    assert_eq!(titles(&recent.data()["content"]), ["The Film", "The Show"]);
    let film = &recent.data()["content"][0];
    assert_eq!(film["type"], "movie");
    assert_eq!(film["metadataKey"], "815");
    assert_eq!(film["nowPlayingBottom"], 2019);
    assert_eq!(film["user"], "");
    assert_eq!(
        film["imageURL"],
        format!(
            "api/v2/homepage/image?source={}&type=Primary&img=815&height=600&width=400&key=815-list",
            kind.path
        )
    );
    let series = &recent.data()["content"][1];
    assert_eq!(series["type"], "tv");
    assert_eq!(series["nowPlayingImageType"], "Thumb");
    let asked: Vec<String> = server.seen().into_iter().map(|seen| seen.path).collect();
    assert_eq!(
        asked,
        [
            "/Users".to_owned(),
            format!(
                "/Users/1/Items/Latest?EnableImages=true&Limit=5&IsPlayed=false&{fields}{types}"
            ),
        ]
    );

    // The user "Zed" of the server played the film.
    let user = seed::logged_in(database, &app, &format!("zed{}", kind.path), 4).await;
    let other = seed::logged_in(database, &app, "zed", 4).await;
    let recent = call(&app, As::User(&other), "GET", &path, None).await;
    assert_eq!(recent.status, StatusCode::OK, "{}", recent.body);
    assert_eq!(titles(&recent.data()["content"]), ["The Show"]);
    let recent = call(&app, As::User(&user), "GET", &path, None).await;
    assert_eq!(titles(&recent.data()["content"]), ["The Film", "The Show"]);
}

#[tokio::test]
async fn recent_items_are_those_that_the_user_of_the_server_did_not_play() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        recent_items_are_those_of_the_user(database, EMBY).await;
    })
    .await;
    on_each_engine(async |database| {
        seed::installed(database).await;
        recent_items_are_those_of_the_user(database, JELLYFIN).await;
    })
    .await;
}

async fn details_give_one_item(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let app = installed_app_with(database, &settings(kind, &server.url(), &[]));
    let path = format!("/homepage/{}/metadata", kind.path);

    let details = call(&app, As::Admin, "POST", &path, Some(json!({"key": "815"}))).await;
    assert_eq!(details.status, StatusCode::OK, "{}", details.body);
    let content = &details.data()["content"];
    assert_eq!(content.as_array().map(Vec::len), Some(1));
    let film = &content[0];
    assert_eq!(film["title"], "The Film");
    assert_eq!(film["uid"], "815");
    assert_eq!(
        film["metadata"],
        json!({
            "guid": "815",
            "summary": "A film.",
            "rating": "7.5",
            "duration": "60000000000",
            "originallyAvailableAt": "2019-05-01T00:00:00.0000000Z",
            "year": 2019,
            "tagline": "One film.",
            "genres": ["Drama"],
            "actors": [{
                "name": "An Actor",
                "role": "The Hero",
                "thumb": format!("api/v2/homepage/image?source={}&type=Primary&img=9&height=450&width=300&key=9-cast", kind.path),
            }],
        })
    );
    assert_eq!(
        server.seen()[1].path,
        "/Users/1/Items/815?EnableImages=true&Limit=50&IsPlayed=false&Fields=Overview%2CPeople%2CGenres%2CCriticRating%2CStudios%2CTaglines"
    );

    // A key that is no id could name another path of the server.
    for key in [
        json!("815/../../Sessions"),
        json!(""),
        json!("0"),
        Value::Null,
    ] {
        let refused = call(&app, As::Admin, "POST", &path, Some(json!({"key": key}))).await;
        assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            refused.message(),
            json!(format!("{} Metadata key is not defined", kind.name))
        );
    }
    assert_eq!(server.seen().len(), 2);

    // The server knows no item with this id.
    let unknown = call(&app, As::Admin, "POST", &path, Some(json!({"key": "999"}))).await;
    assert_eq!(unknown.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        unknown.message(),
        json!(format!("{} Error Occurred", kind.name))
    );
}

#[tokio::test]
async fn details_give_one_item_and_a_key_must_be_an_id() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        details_give_one_item(database, EMBY).await;
        details_give_one_item(database, JELLYFIN).await;
    })
    .await;
}

async fn parts_are_refused(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let url = server.url();
    let routes = [
        ("GET", format!("/homepage/{}/streams", kind.path)),
        ("GET", format!("/homepage/{}/recent", kind.path)),
        ("POST", format!("/homepage/{}/metadata", kind.path)),
    ];
    let body = |method: &str| (method == "POST").then(|| json!({"key": "815"}));
    // Each case: the settings that differ, who asks, and the answer of each route.
    let conflict = StatusCode::CONFLICT;
    let unprocessable = StatusCode::UNPROCESSABLE_ENTITY;
    let unauthorized = StatusCode::UNAUTHORIZED;
    let not_approved = "User not approved to view this homepage item".to_owned();
    let off = format!("{} module is not enabled", kind.enabled);

    let all_off = installed_app_with(
        database,
        &settings(kind, &url, &[(kind.enabled, json!(false))]),
    );
    for (method, path) in &routes {
        let refused = call(&all_off, As::Admin, method, path, body(method)).await;
        assert_eq!(refused.status, conflict, "{path}");
        assert_eq!(refused.message(), json!(off), "{path}");
    }

    // The switch and the level of one part.
    let parts_off = installed_app_with(
        database,
        &settings(
            kind,
            &url,
            &[
                (kind.streams.0, json!(false)),
                (kind.recent.1, json!("1")),
                (kind.auth, json!("999")),
            ],
        ),
    );
    let user = seed::logged_in(database, &parts_off, &format!("pat{}", kind.path), 4).await;
    let refused = call(&parts_off, As::Admin, "GET", &routes[0].1, None).await;
    assert_eq!(refused.status, conflict);
    assert_eq!(
        refused.message(),
        json!(format!("{} module is not enabled", kind.streams.0))
    );
    let refused = call(&parts_off, As::User(&user), "GET", &routes[1].1, None).await;
    assert_eq!(refused.status, unauthorized);
    assert_eq!(refused.message(), json!(not_approved));
    // The details need the level of the server only, and a guest has it here.
    let details = call(&parts_off, As::Guest, "POST", &routes[2].1, body("POST")).await;
    assert_eq!(details.status, StatusCode::OK, "{}", details.body);
    assert_eq!(
        call(&parts_off, As::User(&user), "GET", "/homepage/items", None)
            .await
            .data(),
        json!([])
    );

    // A missing address, and a missing key.
    for missing in [kind.url, kind.token] {
        let app = installed_app_with(database, &settings(kind, &url, &[(missing, json!(""))]));
        for (method, path) in &routes {
            let refused = call(&app, As::Admin, method, path, body(method)).await;
            assert_eq!(refused.status, unprocessable, "{path}");
            assert_eq!(
                refused.message(),
                json!(format!("{missing} was not supplied")),
                "{path}"
            );
        }
    }

    // A wrong key, and a server that is away.
    let wrong = installed_app_with(
        database,
        &settings(kind, &url, &[(kind.token, json!("another-key"))]),
    );
    let away = installed_app_with(database, &settings(kind, "http://127.0.0.1:1", &[]));
    for (method, path) in &routes {
        let refused = call(&wrong, As::Admin, method, path, body(method)).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
        assert_eq!(
            refused.message(),
            json!(format!("{} Error Occurred", kind.name)),
            "{path}"
        );
        let refused = call(&away, As::Admin, method, path, body(method)).await;
        assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR, "{path}");
        let message = refused.message().as_str().unwrap_or_default().to_owned();
        assert!(
            message.starts_with("the server cannot be reached"),
            "{message}"
        );
        // The text names no address.
        assert!(!message.contains("127.0.0.1"), "{message}");
    }
}

#[tokio::test]
async fn parts_are_refused_as_the_php_application_refuses_them() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        parts_are_refused(database, EMBY).await;
        parts_are_refused(database, JELLYFIN).await;
    })
    .await;
}

async fn connection_is_tested(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let url = server.url();
    let path = format!("/test/{}", kind.path);

    let app = installed_app_with(database, &settings(kind, &url, &[]));
    let fine = call(&app, As::Admin, "POST", &path, None).await;
    assert_eq!(fine.status, StatusCode::OK, "{}", fine.body);
    assert_eq!(fine.message(), json!("API Connection succeeded"));
    let seen = &server.seen()[0];
    assert_eq!(seen.path, "/Users");
    assert_eq!(seen.header(kind.header.0), Some(kind.header.1));

    // A test is for Admins and Co-Admins.
    let user = seed::logged_in(database, &app, &format!("kim{}", kind.path), 4).await;
    let refused = call(&app, As::User(&user), "POST", &path, None).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);

    let wrong = installed_app_with(
        database,
        &settings(kind, &url, &[(kind.token, json!("another-key"))]),
    );
    let refused = call(&wrong, As::Admin, "POST", &path, None).await;
    assert_eq!(refused.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        refused.message(),
        json!(format!("{} Connection Error", kind.name))
    );

    // Without an address, each server has the text of the PHP application.
    let empty = installed_app_with(database, &settings(kind, &url, &[(kind.url, json!(""))]));
    let refused = call(&empty, As::Admin, "POST", &path, None).await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    let text = if kind.path == "emby" {
        "embyURL was not supplied"
    } else {
        "Jellyfin URL is not defined"
    };
    assert_eq!(refused.message(), json!(text));
    let empty = installed_app_with(database, &settings(kind, &url, &[(kind.token, json!(""))]));
    let refused = call(&empty, As::Admin, "POST", &path, None).await;
    assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
    let text = if kind.path == "emby" {
        "embyToken was not supplied"
    } else {
        "Jellyfin Token is not defined"
    };
    assert_eq!(refused.message(), json!(text));

    // The test of Emby needs the switch of Emby on the homepage. The one of Jellyfin
    // does not.
    let off = installed_app_with(
        database,
        &settings(kind, &url, &[(kind.enabled, json!(false))]),
    );
    let answer = call(&off, As::Admin, "POST", &path, None).await;
    if kind.path == "emby" {
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepageEmbyEnabled module is not enabled")
        );
    } else {
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    }
}

#[tokio::test]
async fn connection_tests_say_whether_the_server_answered() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        connection_is_tested(database, EMBY).await;
        connection_is_tested(database, JELLYFIN).await;
    })
    .await;
}

async fn pictures_are_fetched_and_kept(database: &Database, kind: Kind) {
    let server = stand_in(kind).await;
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let app = installed_app_in(database, folder.path());
    let saved = call(
        &app,
        As::Admin,
        "PUT",
        "/config",
        Some(json!({
            kind.enabled: true,
            kind.streams.0: true,
            kind.url: server.url(),
            kind.token: MEDIA_SERVER_KEY,
        })),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
    let source = kind.path;
    let path = format!(
        "/homepage/image?source={source}&type=Primary&img=815&height=600&width=400&key=815-list"
    );
    let picture = async |who: As<'_>, path: &str| call(&app, who, "GET", path, None).await;

    let first = picture(As::Admin, &path).await;
    assert_eq!(first.status, StatusCode::OK, "{}", first.body);
    let header = |answer: &Answer, name: &str| {
        answer
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    assert_eq!(
        header(&first, "content-type").as_deref(),
        Some("image/jpeg")
    );
    let seen = server.seen();
    assert_eq!(
        seen[0].path,
        "/Items/815/Images/Primary?maxHeight=600&maxWidth=400"
    );
    // The server gives pictures without the key, so the key is not sent.
    assert_eq!(seen[0].header(kind.header.0), None);
    // The name comes from the picture that was fetched (D-153).
    let name = organizr_integrations::emby::picture_name("815", "Primary", (400, 600), "815-list")
        .expect("a poster key names a picture");
    let file = folder
        .path()
        .join("cache")
        .join(format!("{source}-{name}.jpg"));
    assert_eq!(std::fs::read(file).expect("the file is there"), JPEG);

    // The second time, the picture comes from the cache. A `$` asks for it anew.
    picture(As::Admin, &path).await;
    assert_eq!(server.seen().len(), 1);
    let again = picture(As::Admin, &format!("{path}$abc")).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(server.seen().len(), 2);

    // An item names the picture that the cache holds by its file.
    let streams = call(
        &app,
        As::Admin,
        "GET",
        &format!("/homepage/{source}/streams"),
        None,
    )
    .await;
    assert_eq!(streams.status, StatusCode::OK, "{}", streams.body);
    assert_eq!(
        streams.data()["content"][1]["imageURL"],
        format!("data/cache/{source}-{name}.jpg")
    );
    let asked = server.seen().len();

    // A guest has no level for the server. An id that is a path, a type that is not
    // known, a size that is too large and a key that could leave the folder are
    // refused.
    let refused = picture(As::Guest, &path).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    for wrong in [
        format!(
            "/homepage/image?source={source}&type=Primary&img=815/../../Users&height=600&width=400&key=1-list"
        ),
        format!(
            "/homepage/image?source={source}&type=Primary&img=https://evil.example.net/a.jpg&height=600&width=400&key=1-list"
        ),
        format!("/homepage/image?source={source}&type=..&img=815&height=600&width=400&key=1-list"),
        format!("/homepage/image?source={source}&type=&img=815&height=600&width=400&key=1-list"),
        format!(
            "/homepage/image?source={source}&type=Primary&img=815&height=60000&width=400&key=1-list"
        ),
        format!(
            "/homepage/image?source={source}&type=Primary&img=815&height=600&width=400&key=..%2Fsecret"
        ),
    ] {
        let refused = picture(As::Admin, &wrong).await;
        assert_eq!(refused.status, StatusCode::NOT_FOUND, "{wrong}");
    }
    assert_eq!(server.seen().len(), asked);
}

#[tokio::test]
async fn pictures_of_the_server_are_fetched_in_a_size_and_kept() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        pictures_are_fetched_and_kept(database, EMBY).await;
        pictures_are_fetched_and_kept(database, JELLYFIN).await;
    })
    .await;
}
