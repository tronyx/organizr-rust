//! The download clients Deluge, rTorrent, uTorrent and JDownloader on the homepage,
//! against stand-ins for them.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{
    self, DELUGE_PASSWORD, JDOWNLOADER_LOGIN, RTORRENT_LOGIN, UTORRENT_LOGIN, UTORRENT_TOKEN,
};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

type Stored = Vec<(&'static str, Value)>;

/// The values of one field of the entries of a queue.
fn column(content: &Value, field: &str) -> Vec<String> {
    content["queueItems"]
        .as_array()
        .expect("a list of entries")
        .iter()
        .map(|entry| entry[field].as_str().unwrap_or("?").to_owned())
        .collect()
}

/// Asks a route as the Admin, with the given settings.
async fn get(database: &Database, path: &str, settings: &[(&'static str, Value)]) -> Answer {
    let app = installed_app_with(database, settings);
    call(&app, As::Admin, "GET", path, None).await
}

/// Runs the connection test of a service as the Admin, with the given settings.
async fn test(database: &Database, service: &str, settings: &[(&'static str, Value)]) -> Answer {
    let app = installed_app_with(database, settings);
    call(&app, As::Admin, "POST", &format!("/test/{service}"), None).await
}

fn succeeded(answer: &Answer) {
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.message(), json!("API Connection succeeded"));
}

/// The kinds of the items that the Admin sees.
async fn kinds(database: &Database, settings: &[(&'static str, Value)]) -> Vec<String> {
    get(database, "/homepage/items", settings)
        .await
        .data()
        .as_array()
        .expect("a list of items")
        .iter()
        .map(|item| item["kind"].as_str().unwrap_or("?").to_owned())
        .collect()
}

/// Settings with more settings after them.
fn with(mut settings: Stored, more: &[(&'static str, Value)]) -> Stored {
    settings.extend(more.iter().cloned());
    settings
}

#[tokio::test]
async fn deluge_torrents_come_as_the_plugin_gives_them() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::deluge().await;
        let settings = |password: &str| -> Stored {
            vec![
                ("homepageDelugeEnabled", json!(true)),
                ("homepageDelugeAuth", json!("4")),
                ("delugeURL", json!(format!("{}/", service.url()))),
                ("delugePassword", json!(password)),
            ]
        };
        let path = "/homepage/deluge/queue";

        let app = installed_app_with(database, &settings(DELUGE_PASSWORD));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let answer = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content["historyItems"], json!(false));
        assert_eq!(
            column(content, "name"),
            ["The.Show.S02E03", "An.Old.Film", "A.Paused.Film"]
        );
        assert_eq!(content["queueItems"][0]["progress"], json!(40.5));
        // The status of the tracker is left out unless its switch is off.
        assert_eq!(content["queueItems"][0]["tracker_status"], json!(""));

        // A login with the password, and then the list with the cookie of the login.
        let seen = service.seen();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            (seen[0].method.as_str(), seen[0].path.as_str()),
            ("POST", "/json")
        );
        assert_eq!(
            seen[0].json(),
            json!({"id": 0, "method": "auth.login", "params": [DELUGE_PASSWORD]})
        );
        assert_eq!(seen[1].json()["id"], json!(1));
        assert_eq!(seen[1].json()["method"], json!("webapi.get_torrents"));
        assert_eq!(seen[1].json()["params"][0], Value::Null);
        assert!(seen[1].json()["params"][1].as_str().is_some_and(|fields| {
            fields.starts_with("comment, ") && fields.ends_with(", tracker_status")
        }));
        assert_eq!(
            seen[1].header("cookie"),
            Some("_session_id=made-up-deluge-session")
        );
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrderdeluge", "kind": "deluge", "id": ""}])
        );

        let hiding = with(
            settings(DELUGE_PASSWORD),
            &[
                ("delugeHideSeeding", json!(true)),
                ("delugeHideStatus", json!(false)),
            ],
        );
        let answer = get(database, path, &hiding).await;
        let content = &answer.data()["content"];
        assert_eq!(column(content, "name"), ["The.Show.S02E03"]);
        assert_eq!(
            content["queueItems"][0]["tracker_status"],
            json!("Announce OK")
        );

        // Deluge refuses a wrong password. The answer does not hold the password.
        let wrong = get(database, path, &settings("a wrong password")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("Login failed"));
        assert!(!wrong.body.contains("a wrong password"), "{}", wrong.body);

        // Nothing listens at this address. The answer names no address.
        let mut away = settings(DELUGE_PASSWORD);
        away[2] = ("delugeURL", json!("http://127.0.0.1:1"));
        let away = get(database, path, &away).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!away.body.contains("127.0.0.1"), "{}", away.body);
    })
    .await;
}

#[tokio::test]
async fn deluge_has_the_refusals_of_every_item_and_a_connection_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::deluge().await;
        let path = "/homepage/deluge/queue";
        let settings = |password: &str| -> Stored {
            vec![
                ("homepageDelugeEnabled", json!(true)),
                ("delugeURL", json!(service.url())),
                ("delugePassword", json!(password)),
            ]
        };

        let off = get(database, path, &[("delugeURL", json!(service.url()))]).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageDelugeEnabled module is not enabled")
        );
        let no_address = get(database, path, &[("homepageDelugeEnabled", json!(true))]).await;
        assert_eq!(no_address.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(no_address.message(), json!("delugeURL was not supplied"));
        // The level of the item is the one of a Co-Admin at the start.
        let app = installed_app_with(database, &settings(DELUGE_PASSWORD));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        for who in [As::Guest, As::User(&user)] {
            let refused = call(&app, who, "GET", path, None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
            assert_eq!(
                refused.message(),
                json!("User not approved to view this homepage item")
            );
        }
        assert_eq!(service.seen().len(), 0);

        succeeded(&test(database, "deluge", &settings(DELUGE_PASSWORD)).await);
        let wrong = test(database, "deluge", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("Login failed"));
        // The test has the checks of the item, as in the PHP application.
        let off = test(database, "deluge", &[("delugeURL", json!(service.url()))]).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        let none = test(
            database,
            "deluge",
            &[("homepageDelugeEnabled", json!(true))],
        )
        .await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("delugeURL was not supplied"));
    })
    .await;
}

#[tokio::test]
async fn rtorrent_torrents_come_with_the_fields_of_php_in_the_order_of_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::rtorrent().await;
        let settings = |password: &str| -> Stored {
            vec![
                ("homepagerTorrentEnabled", json!(true)),
                ("homepagerTorrentAuth", json!("4")),
                ("rTorrentURL", json!(format!("{}/", service.url()))),
                ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
                ("rTorrentPassword", json!(password)),
            ]
        };
        let path = "/homepage/rtorrent/queue";

        let app = installed_app_with(database, &settings(RTORRENT_LOGIN.1));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let answer = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content["historyItems"], json!(false));
        // The oldest torrent is first: the order of the setting at the start.
        assert_eq!(
            column(content, "name"),
            ["An.Old.Film", "A.Closed.Film", "The.Show.S02E03"]
        );
        assert_eq!(
            column(content, "status"),
            ["Seeding", "Finished", "Downloading"]
        );
        // Every number of rTorrent is a text, as in the PHP application.
        assert_eq!(
            content["queueItems"][2],
            json!({
                "name": "The.Show.S02E03",
                "base": "/downloads/The.Show.S02E03",
                "upTotal": "5000",
                "size": "1000000",
                "downTotal": "400000",
                "downloaded": "400000",
                "connectionState": "leech",
                "leech": "1000",
                "seed": "10",
                "date": "300",
                "state": "on",
                "group": "",
                "hash": "HASH300",
                "complete": "no",
                "ratio": "5",
                "label": "tv",
                "status": "Downloading",
                "temp": "0 - 0 - 0",
                "custom": " - tv - ",
                "custom2": " -  - ",
            })
        );

        // The login goes to rTorrent only after rTorrent asked for it.
        let seen = service.seen();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            (seen[0].method.as_str(), seen[0].path.as_str()),
            ("POST", "/RPC2")
        );
        assert_eq!(seen[0].header("authorization"), None);
        assert_eq!(seen[1].header("authorization"), Some(RTORRENT_LOGIN.2));
        assert!(
            seen[1].body.contains(
                "<methodName>d.multicall2</methodName><params>\
                 <param><value><string></string></value></param>\
                 <param><value><string>main</string></value></param>\
                 <param><value><string>d.name=</string></value></param>"
            ),
            "{}",
            seen[1].body
        );
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrderrTorrent", "kind": "rtorrent", "id": ""}])
        );
    })
    .await;
}

#[tokio::test]
async fn rtorrent_torrents_follow_the_settings() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::rtorrent().await;
        let settings = || -> Stored {
            vec![
                ("homepagerTorrentEnabled", json!(true)),
                ("rTorrentURL", json!(service.url())),
                ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
                ("rTorrentPassword", json!(RTORRENT_LOGIN.1)),
            ]
        };
        let path = "/homepage/rtorrent/queue";

        let names = async |more: &[(&'static str, Value)]| {
            let answer = get(database, path, &with(settings(), more)).await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            column(&answer.data()["content"], "name")
        };
        assert_eq!(
            names(&[("rTorrentHideSeeding", json!(true))]).await,
            ["A.Closed.Film", "The.Show.S02E03"]
        );
        assert_eq!(
            names(&[("rTorrentHideCompleted", json!(true))]).await,
            ["An.Old.Film", "The.Show.S02E03"]
        );
        // A torrent without a label stays.
        assert_eq!(
            names(&[("rTorrentIgnoreLabel", json!("TV,private"))]).await,
            ["An.Old.Film", "A.Closed.Film"]
        );
        assert_eq!(
            names(&[
                ("rTorrentSortOrder", json!("named")),
                ("rTorrentLimit", json!("2")),
            ])
            .await,
            ["The.Show.S02E03", "An.Old.Film"]
        );
    })
    .await;
}

#[tokio::test]
async fn rtorrent_is_asked_at_the_address_of_the_settings_and_answers_a_digest_challenge() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::rtorrent().await;
        let path = "/homepage/rtorrent/queue";
        let login = |password: &str| -> Stored {
            vec![
                ("homepagerTorrentEnabled", json!(true)),
                ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
                ("rTorrentPassword", json!(password)),
            ]
        };

        // The second address stands after the address, and gets no `/RPC2`.
        let digest = with(
            login(RTORRENT_LOGIN.1),
            &[
                ("rTorrentURL", json!(service.url())),
                ("rTorrentURLOverride", json!("/digest/RPC2")),
            ],
        );
        let answer = get(database, path, &digest).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(column(&answer.data()["content"], "name").len(), 3);
        let seen = service.seen();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].path, "/digest/RPC2");
        let sent = seen[1].header("authorization").unwrap_or_default();
        assert!(sent.starts_with("Digest username=\"rtorrent\""), "{sent}");
        assert!(sent.contains("qop=auth, nc=00000001"), "{sent}");
        // The password itself does not go to the server.
        assert!(!sent.contains(RTORRENT_LOGIN.1), "{sent}");

        // A second address that holds the address stands in its place.
        let whole = with(
            login(RTORRENT_LOGIN.1),
            &[
                ("rTorrentURL", json!(service.url())),
                (
                    "rTorrentURLOverride",
                    json!(format!("{}/RPC2", service.url())),
                ),
            ],
        );
        let answer = get(database, path, &whole).await;
        assert_eq!(column(&answer.data()["content"], "name").len(), 3);
        assert_eq!(
            service.seen().last().map(|seen| seen.path.clone()),
            Some("/RPC2".to_owned())
        );

        // rTorrent refuses a wrong password, and the content is then `false`.
        let wrong = with(login("wrong"), &[("rTorrentURL", json!(service.url()))]);
        let answer = get(database, path, &wrong).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!({"content": false}));
        // Without a name, no login is sent at all.
        let before = service.seen().len();
        let nameless = [
            ("homepagerTorrentEnabled", json!(true)),
            ("rTorrentURL", json!(service.url())),
            ("rTorrentPassword", json!(RTORRENT_LOGIN.1)),
        ];
        let answer = get(database, path, &nameless).await;
        assert_eq!(answer.data(), json!({"content": false}));
        assert_eq!(service.seen().len(), before + 1);

        // A fault of rTorrent is no list.
        let fault = with(
            login(RTORRENT_LOGIN.1),
            &[("rTorrentURL", json!(format!("{}/no-list", service.url())))],
        );
        let answer = get(database, path, &fault).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!({"content": false}));
        assert_eq!(
            service.seen().last().map(|seen| seen.path.clone()),
            Some("/no-list/RPC2".to_owned())
        );

        // Nothing listens at this address. The answer names no address.
        let away = with(
            login(RTORRENT_LOGIN.1),
            &[("rTorrentURL", json!("http://127.0.0.1:1"))],
        );
        let away = get(database, path, &away).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!away.body.contains("127.0.0.1"), "{}", away.body);
    })
    .await;
}

#[tokio::test]
async fn rtorrent_asks_for_its_address_first_and_has_a_connection_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::rtorrent().await;
        let path = "/homepage/rtorrent/queue";
        let settings = |password: &str| -> Stored {
            vec![
                ("rTorrentURL", json!(service.url())),
                ("rTorrentUsername", json!(RTORRENT_LOGIN.0)),
                ("rTorrentPassword", json!(password)),
            ]
        };

        // Without an address, everybody gets this answer, as in the PHP application.
        let app = installed_app_with(database, &[]);
        for who in [As::Guest, As::Admin] {
            let none = call(&app, who, "GET", path, None).await;
            assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(none.message(), json!("rTorrent URL is not defined"));
        }
        let off = get(database, path, &settings(RTORRENT_LOGIN.1)).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepagerTorrentEnabled module is not enabled")
        );
        let on = with(
            settings(RTORRENT_LOGIN.1),
            &[("homepagerTorrentEnabled", json!(true))],
        );
        let app = installed_app_with(database, &on);
        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), 0);
        // The item is listed without an address too, as in the PHP application.
        let listed = kinds(database, &[("homepagerTorrentEnabled", json!(true))]).await;
        assert!(listed.contains(&"rtorrent".to_owned()), "{listed:?}");

        succeeded(&test(database, "rtorrent", &settings(RTORRENT_LOGIN.1)).await);
        assert!(
            service
                .seen()
                .last()
                .is_some_and(|seen| seen.body.contains("system.listMethods"))
        );
        let wrong = test(database, "rtorrent", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("rTorrent error occurred"));
        let none = test(database, "rtorrent", &[]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("rTorrent URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn utorrent_torrents_come_with_names_for_their_values() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::utorrent().await;
        let settings = |password: &str| -> Stored {
            vec![
                ("homepageuTorrentEnabled", json!(true)),
                ("homepageuTorrentAuth", json!("4")),
                ("uTorrentURL", json!(format!("{}/", service.url()))),
                ("uTorrentUsername", json!(UTORRENT_LOGIN.0)),
                ("uTorrentPassword", json!(password)),
            ]
        };
        let path = "/homepage/utorrent/queue";

        let app = installed_app_with(database, &settings(UTORRENT_LOGIN.1));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let answer = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        // The PHP application sends no history for uTorrent.
        assert_eq!(content.as_object().map(serde_json::Map::len), Some(1));
        assert_eq!(
            column(content, "Name"),
            ["The.Show.S02E03", "An.Old.Film", "A.Finished.Film"]
        );
        assert_eq!(
            column(content, "Status"),
            ["Downloading", "Seeding", "Finished"]
        );
        assert_eq!(column(content, "Percent"), ["40.0%", "100.0%", "100.0%"]);
        let first = &content["queueItems"][0];
        assert_eq!(first["Hash"], json!("HASH-OF-The.Show.S02E03"));
        assert_eq!(first["Size"], json!(1_000_000));
        assert_eq!(first["Labels"], json!("tv"));
        assert_eq!(first["Message"], json!("Downloading 40.0 %"));
        assert_eq!(first["Unknown28"], json!("ABCD"));

        // The token page first, and then the list with the token and the cookie.
        let seen = service.seen();
        assert_eq!(seen.len(), 2);
        assert_eq!(
            (seen[0].method.as_str(), seen[0].path.as_str()),
            ("POST", "/gui/token.html")
        );
        assert_eq!(seen[0].header("authorization"), Some(UTORRENT_LOGIN.2));
        assert_eq!(
            (seen[1].method.as_str(), seen[1].path.as_str()),
            (
                "GET",
                format!("/gui/?token={UTORRENT_TOKEN}&list=1").as_str()
            )
        );
        assert_eq!(seen[1].header("cookie"), Some("GUID=made-up-guid"));
        assert_eq!(seen[1].header("authorization"), Some(UTORRENT_LOGIN.2));
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrderuTorrent", "kind": "utorrent", "id": ""}])
        );

        let hiding = with(
            settings(UTORRENT_LOGIN.1),
            &[
                ("uTorrentHideSeeding", json!(true)),
                ("uTorrentHideCompleted", json!(true)),
            ],
        );
        let answer = get(database, path, &hiding).await;
        assert_eq!(
            column(&answer.data()["content"], "Name"),
            ["The.Show.S02E03"]
        );

        // uTorrent gives no token for a wrong password.
        let wrong = get(database, path, &settings("a wrong password")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!("uTorrent Connect Function - Error: Could not get token")
        );
        assert!(!wrong.body.contains("a wrong password"), "{}", wrong.body);

        // Nothing listens at this address. The answer names no address.
        let mut away = settings(UTORRENT_LOGIN.1);
        away[2] = ("uTorrentURL", json!("http://127.0.0.1:1"));
        let away = get(database, path, &away).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!away.body.contains("127.0.0.1"), "{}", away.body);
    })
    .await;
}

#[tokio::test]
async fn utorrent_has_the_refusals_of_every_item_and_a_connection_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::utorrent().await;
        let path = "/homepage/utorrent/queue";
        let settings = |password: &str| -> Stored {
            vec![
                ("uTorrentURL", json!(service.url())),
                ("uTorrentUsername", json!(UTORRENT_LOGIN.0)),
                ("uTorrentPassword", json!(password)),
            ]
        };

        let off = get(database, path, &settings(UTORRENT_LOGIN.1)).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageuTorrentEnabled module is not enabled")
        );
        let no_address = get(database, path, &[("homepageuTorrentEnabled", json!(true))]).await;
        assert_eq!(no_address.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(no_address.message(), json!("uTorrentURL was not supplied"));
        let on = with(
            settings(UTORRENT_LOGIN.1),
            &[("homepageuTorrentEnabled", json!(true))],
        );
        let app = installed_app_with(database, &on);
        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), 0);

        succeeded(&test(database, "utorrent", &settings(UTORRENT_LOGIN.1)).await);
        let wrong = test(database, "utorrent", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!("uTorrent Connect Function - Error: Could not get token")
        );
        let none = test(database, "utorrent", &[]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("uTorrent URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn jdownloader_packages_come_in_four_lists_with_the_states() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jdownloader().await;
        let settings = |password: &str| -> Stored {
            vec![
                ("homepageJdownloaderEnabled", json!(true)),
                ("homepageJdownloaderAuth", json!("4")),
                ("jdownloaderURL", json!(format!("{}/", service.url()))),
                ("jdownloaderUsername", json!(JDOWNLOADER_LOGIN.0)),
                ("jdownloaderPassword", json!(password)),
            ]
        };
        let path = "/homepage/jdownloader/queue";

        let app = installed_app_with(database, &settings(JDOWNLOADER_LOGIN.1));
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let answer = call(&app, As::User(&user), "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!({"content": {
                "queueItems": [{
                    "name": "A.Package", "speed": "1.2 MB/s", "size": "4.00 GB",
                    "done": "1.00 GB", "eta": "40m:00s", "percentage": "25", "enabled": true,
                }],
                "grabberItems": [{"name": "A.New.Package", "size": "700.00 MB"}],
                "encryptedItems": [],
                "offlineItems": [{"name": "A.Lost.Package"}],
                "$status": ["RUNNING", false, false],
            }})
        );
        // The address of the settings is asked as it is.
        let seen = service.seen();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            (seen[0].method.as_str(), seen[0].path.as_str()),
            ("GET", "/")
        );
        assert_eq!(seen[0].header("authorization"), Some(JDOWNLOADER_LOGIN.2));
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrderjdownloader", "kind": "jdownloader", "id": ""}])
        );

        // The service refuses a wrong password, and the content is then `false`.
        let wrong = get(database, path, &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::OK);
        assert_eq!(wrong.data(), json!({"content": false}));
        // Without a name and a password, no login is sent.
        let before = service.seen().len();
        let open = [
            ("homepageJdownloaderEnabled", json!(true)),
            ("jdownloaderURL", json!(service.url())),
        ];
        let answer = get(database, path, &open).await;
        assert_eq!(answer.data(), json!({"content": false}));
        assert_eq!(service.seen()[before].header("authorization"), None);

        // Nothing listens at this address. The answer names no address.
        let mut away = settings(JDOWNLOADER_LOGIN.1);
        away[2] = ("jdownloaderURL", json!("http://127.0.0.1:1"));
        let away = get(database, path, &away).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!away.body.contains("127.0.0.1"), "{}", away.body);
    })
    .await;
}

#[tokio::test]
async fn jdownloader_has_the_refusals_of_every_item_and_a_connection_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::jdownloader().await;
        let path = "/homepage/jdownloader/queue";
        let settings = |password: &str| -> Stored {
            vec![
                ("jdownloaderURL", json!(service.url())),
                ("jdownloaderUsername", json!(JDOWNLOADER_LOGIN.0)),
                ("jdownloaderPassword", json!(password)),
            ]
        };

        let off = get(database, path, &settings(JDOWNLOADER_LOGIN.1)).await;
        assert_eq!(off.status, StatusCode::CONFLICT);
        assert_eq!(
            off.message(),
            json!("homepageJdownloaderEnabled module is not enabled")
        );
        let enabled = [("homepageJdownloaderEnabled", json!(true))];
        let no_address = get(database, path, &enabled).await;
        assert_eq!(no_address.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            no_address.message(),
            json!("jdownloaderURL was not supplied")
        );
        let app = installed_app_with(database, &with(settings(JDOWNLOADER_LOGIN.1), &enabled));
        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), 0);

        succeeded(&test(database, "jdownloader", &settings(JDOWNLOADER_LOGIN.1)).await);
        let wrong = test(database, "jdownloader", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("JDownloader Error Occurred"));
        let none = test(database, "jdownloader", &[]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("JDownloader URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn only_an_admin_or_a_co_admin_runs_the_tests_of_these_clients() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let user = seed::logged_in(database, &app, "robin", 4).await;

        for (service, nothing_set_up) in [
            // The test of Deluge has the checks of its item, which is off.
            ("deluge", StatusCode::CONFLICT),
            ("rtorrent", StatusCode::UNPROCESSABLE_ENTITY),
            ("utorrent", StatusCode::UNPROCESSABLE_ENTITY),
            ("jdownloader", StatusCode::UNPROCESSABLE_ENTITY),
        ] {
            let path = format!("/test/{service}");
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, "POST", &path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{service}");
            }
            let answer = call(&app, As::User(&boss), "POST", &path, None).await;
            assert_eq!(answer.status, nothing_set_up, "{service}");
        }
    })
    .await;
}

#[tokio::test]
async fn these_clients_share_the_downloader_item_when_their_combine_switch_is_on() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings: Stored = vec![
            ("homepageDelugeEnabled", json!(true)),
            ("delugeURL", json!("http://deluge.example.com")),
            ("homepageOrderdeluge", json!("14")),
            ("homepagerTorrentEnabled", json!(true)),
            ("rTorrentURL", json!("http://rtorrent.example.com")),
            ("homepageOrderrTorrent", json!("11")),
            ("homepageuTorrentEnabled", json!(true)),
            ("uTorrentURL", json!("http://utorrent.example.com")),
            ("homepageOrderuTorrent", json!("12")),
            ("homepageJdownloaderEnabled", json!(true)),
            ("jdownloaderURL", json!("http://jdownloader.example.com")),
            ("homepageOrderjdownloader", json!("13")),
            ("homepageOrderdownloader", json!("20")),
        ];

        // Each client is an item of its own, in the order of the places.
        assert_eq!(
            kinds(database, &settings).await,
            ["rtorrent", "utorrent", "jdownloader", "deluge"]
        );

        let combined = with(
            settings,
            &[
                ("delugeCombine", json!(true)),
                ("rTorrentCombine", json!(true)),
                ("uTorrentCombine", json!(true)),
                ("jdownloaderCombine", json!(true)),
            ],
        );
        let listed = get(database, "/homepage/items", &combined).await;
        assert_eq!(
            listed.data(),
            json!([{
                "key": "homepageOrderdownloader",
                "kind": "downloader",
                "id": "",
                "options": {"members": ["rtorrent", "utorrent", "jdownloader", "deluge"]},
            }])
        );
    })
    .await;
}
