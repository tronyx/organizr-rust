//! The download clients of the homepage, against stand-ins for SABnzbd and
//! qBittorrent.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{
    self, NZBGET_LOGIN, QBITTORRENT_LOGIN, SABNZBD_KEY, TRANSMISSION_LOGIN,
};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

#[tokio::test]
async fn sabnzbd_queue_and_history_come_as_sabnzbd_gives_them() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::sabnzbd().await;
        let settings = |key: &str| {
            vec![
                ("homepageSabnzbdEnabled", json!(true)),
                ("homepageSabnzbdAuth", json!("4")),
                ("sabnzbdURL", json!(service.url())),
                ("sabnzbdToken", json!(key)),
            ]
        };
        let app = installed_app_with(database, &settings(SABNZBD_KEY));
        let user = seed::logged_in(database, &app, "zed", 4).await;

        let answer = call(
            &app,
            As::User(&user),
            "GET",
            "/homepage/sabnzbd/queue",
            None,
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(
            content["queueItems"]["queue"]["slots"][0]["filename"],
            "The.Show.S02E03.1080p"
        );
        assert_eq!(
            content["historyItems"]["history"]["slots"][0]["name"],
            "The.Film.2020.1080p"
        );
        let asked: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(
            asked,
            [
                format!("/api?mode=queue&output=json&apikey={SABNZBD_KEY}"),
                format!("/api?mode=history&limit=100&output=json&apikey={SABNZBD_KEY}"),
            ]
        );
        assert_eq!(
            call(&app, As::User(&user), "GET", "/homepage/items", None)
                .await
                .data(),
            json!([{"key": "homepageOrdersabnzbd", "kind": "sabnzbd", "id": ""}])
        );

        // SABnzbd refuses a wrong key. The answer then has no content.
        let wrong = installed_app_with(database, &settings("a-wrong-key"));
        let answer = call(&wrong, As::Admin, "GET", "/homepage/sabnzbd/queue", None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data(), json!({"content": false}));

        // Nothing listens at this address. The answer names no key.
        let mut away = settings(SABNZBD_KEY);
        away[2] = ("sabnzbdURL", json!("http://127.0.0.1:1"));
        let away = installed_app_with(database, &away);
        let answer = call(&away, As::Admin, "GET", "/homepage/sabnzbd/queue", None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!answer.body.contains(SABNZBD_KEY), "{}", answer.body);
        assert!(!answer.body.contains("apikey"), "{}", answer.body);

        let guest = call(&app, As::Guest, "GET", "/homepage/sabnzbd/queue", None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn sabnzbd_is_paused_and_resumed_by_admins_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::sabnzbd().await;
        let app = installed_app_with(
            database,
            &[
                ("homepageSabnzbdEnabled", json!(true)),
                ("homepageSabnzbdAuth", json!("4")),
                ("sabnzbdURL", json!(service.url())),
                ("sabnzbdToken", json!(SABNZBD_KEY)),
            ],
        );
        let user = seed::logged_in(database, &app, "zed", 4).await;
        let last = || service.seen().last().map(|seen| seen.path.clone());
        let pause = "/homepage/sabnzbd/queue/pause";
        let resume = "/homepage/sabnzbd/queue/resume";

        let all = call(
            &app,
            As::Admin,
            "POST",
            pause,
            Some(json!({"target": "main"})),
        )
        .await;
        assert_eq!(all.status, StatusCode::OK, "{}", all.body);
        assert_eq!(all.data(), json!({"content": {"status": true}}));
        assert_eq!(
            last(),
            Some(format!("/api?mode=pause&output=json&apikey={SABNZBD_KEY}"))
        );

        let one = json!({"target": "SABnzbd_nzo_one"});
        call(&app, As::Admin, "POST", pause, Some(one.clone())).await;
        assert_eq!(
            last(),
            Some(format!(
                "/api?mode=queue&name=pause&value=SABnzbd_nzo_one&output=json&apikey={SABNZBD_KEY}"
            ))
        );
        call(&app, As::Admin, "POST", resume, Some(one)).await;
        assert_eq!(
            last(),
            Some(format!(
                "/api?mode=queue&name=resume&value=SABnzbd_nzo_one&output=json&apikey={SABNZBD_KEY}"
            ))
        );
        call(&app, As::Admin, "POST", resume, Some(json!({}))).await;
        assert_eq!(
            last(),
            Some(format!("/api?mode=resume&output=json&apikey={SABNZBD_KEY}"))
        );

        // A user sees the queue, and cannot stop it.
        let asked = service.seen().len();
        let refused = call(&app, As::User(&user), "POST", pause, Some(json!({}))).await;
        assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        assert_eq!(service.seen().len(), asked);
    })
    .await;
}

fn names(content: &Value) -> Vec<String> {
    content["queueItems"]
        .as_array()
        .expect("a list of torrents")
        .iter()
        .map(|torrent| torrent["name"].as_str().unwrap_or("?").to_owned())
        .collect()
}

#[tokio::test]
async fn qbittorrent_torrents_come_without_the_ones_that_the_settings_hide() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::qbittorrent().await;
        let settings = |password: &str, more: &[(&'static str, Value)]| {
            let mut settings = vec![
                ("homepageqBittorrentEnabled", json!(true)),
                ("qBittorrentURL", json!(format!("{}/", service.url()))),
                ("qBittorrentUsername", json!(QBITTORRENT_LOGIN.0)),
                ("qBittorrentPassword", json!(password)),
                ("qBittorrentApiVersion", json!("2")),
                ("qBittorrentSortOrder", json!("name")),
            ];
            settings.extend(more.iter().cloned());
            settings
        };
        let path = "/homepage/qbittorrent/queue";

        let app = installed_app_with(database, &settings(QBITTORRENT_LOGIN.1, &[]));
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content["historyItems"], json!(false));
        assert_eq!(
            names(content),
            ["The.Show.S02E03", "An.Old.Film", "A.Finished.Film"]
        );
        assert_eq!(content["queueItems"][0]["progress"], json!(0.4));
        let seen = service.seen();
        assert_eq!(seen[0].path, "/api/v2/auth/login");
        assert_eq!(
            seen[0].form().get("username").map(String::as_str),
            Some("admin")
        );
        assert_eq!(
            seen[1].path,
            "/api/v2/torrents/info?sort=name&reverse=false"
        );

        let hiding = settings(
            QBITTORRENT_LOGIN.1,
            &[
                ("qBittorrentHideSeeding", json!(true)),
                ("qBittorrentHideCompleted", json!(true)),
                ("qBittorrentReverseSorting", json!(true)),
            ],
        );
        let app = installed_app_with(database, &hiding);
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(names(&answer.data()["content"]), ["The.Show.S02E03"]);
        assert!(
            service
                .seen()
                .last()
                .is_some_and(|seen| seen.path.ends_with("&reverse=true"))
        );

        // A wrong password gets no session.
        let wrong = installed_app_with(database, &settings("a wrong password", &[]));
        let refused = call(&wrong, As::Admin, "GET", path, None).await;
        assert_eq!(refused.status, StatusCode::CONFLICT);
        assert_eq!(
            refused.message(),
            json!("qBittorrent Connect Function - Error: Could not get session ID")
        );

        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn nzbget_gives_its_queue_and_its_history() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::nzbget().await;
        let settings = |password: &str| {
            vec![
                ("homepageNzbgetEnabled", json!(true)),
                ("nzbgetURL", json!(format!("{}/", service.url()))),
                ("nzbgetUsername", json!(NZBGET_LOGIN.0)),
                ("nzbgetPassword", json!(password)),
            ]
        };
        let path = "/homepage/nzbget/queue";

        let app = installed_app_with(database, &settings(NZBGET_LOGIN.1));
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(
            content["queueItems"]["result"][0]["NZBName"],
            json!("The.Show.S02E03")
        );
        assert_eq!(
            content["historyItems"]["result"][1]["Status"],
            json!("FAILURE/PAR")
        );
        let paths: Vec<String> = service.seen().into_iter().map(|seen| seen.path).collect();
        assert_eq!(paths, ["/jsonrpc/listgroups", "/jsonrpc/history"]);

        // NZBGet refuses a wrong password, and the content is then `false`.
        let app = installed_app_with(database, &settings("wrong"));
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK);
        assert_eq!(answer.data()["content"], json!(false));
        // Without a password, no login is sent at all.
        let before = service.seen().len();
        let app = installed_app_with(database, &settings(""));
        call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(service.seen()[before].header("authorization"), None);

        // The item has the checks of every item.
        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        let off = installed_app_with(database, &[("nzbgetURL", json!(service.url()))]);
        let answer = call(&off, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        let items = call(
            &installed_app_with(database, &settings(NZBGET_LOGIN.1)),
            As::Admin,
            "GET",
            "/homepage/items",
            None,
        )
        .await;
        assert!(
            items.body.contains("\"kind\":\"nzbget\"")
                || items.body.contains("\"kind\": \"nzbget\""),
            "{}",
            items.body
        );
    })
    .await;
}

#[tokio::test]
async fn transmission_torrents_come_without_the_ones_that_the_settings_hide() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::transmission().await;
        let settings = |password: &str, more: &[(&'static str, Value)]| {
            let mut settings = vec![
                ("homepageTransmissionEnabled", json!(true)),
                ("transmissionURL", json!(format!("{}/", service.url()))),
                ("transmissionUsername", json!(TRANSMISSION_LOGIN.0)),
                ("transmissionPassword", json!(password)),
            ];
            settings.extend(more.iter().cloned());
            settings
        };
        let path = "/homepage/transmission/queue";

        let app = installed_app_with(database, &settings(TRANSMISSION_LOGIN.1, &[]));
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let content = &answer.data()["content"];
        assert_eq!(content["historyItems"], json!(false));
        // The oldest torrent is first.
        assert_eq!(
            names(content),
            ["An.Old.Film", "A.Stopped.Film", "The.Show.S02E03"]
        );
        assert_eq!(content["queueItems"][2]["percentDone"], json!(0.4));
        let seen = service.seen();
        assert_eq!(
            (seen[0].method.as_str(), seen[0].path.as_str()),
            ("GET", "/rpc")
        );
        assert_eq!(seen[1].method, "POST");
        assert_eq!(seen[1].json()["method"], json!("torrent-get"));
        assert_eq!(
            seen[1].json()["arguments"]["fields"]
                .as_array()
                .map(Vec::len),
            Some(12)
        );

        let hiding = settings(
            TRANSMISSION_LOGIN.1,
            &[
                ("transmissionHideSeeding", json!(true)),
                ("transmissionHideCompleted", json!(true)),
            ],
        );
        let app = installed_app_with(database, &hiding);
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(names(&answer.data()["content"]), ["The.Show.S02E03"]);

        // A wrong password gets no session.
        let app = installed_app_with(database, &settings("wrong", &[]));
        let answer = call(&app, As::Admin, "GET", path, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            answer.message(),
            json!("Transmission Connect Function - Error: Could not get session ID")
        );
        let guest = call(&app, As::Guest, "GET", path, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
    })
    .await;
}
