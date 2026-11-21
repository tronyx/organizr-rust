//! The connection tests of the settings pages, against stand-ins for the services.

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
    self, ARR_KEY, NZBGET_LOGIN, OVERSEERR_KEY, PIHOLE_TOKEN, PLEX_SERVER_TOKEN, QBITTORRENT_LOGIN,
    SABNZBD_KEY, TAUTULLI_KEY, TRANSMISSION_LOGIN,
};
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

/// Runs the test of a service as the Admin, with the given settings.
async fn test(database: &Database, service: &str, settings: &[(&str, Value)]) -> Answer {
    let app = installed_app_with(database, settings);
    call(&app, As::Admin, "POST", &format!("/test/{service}"), None).await
}

fn succeeded(answer: &Answer) {
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
    assert_eq!(answer.message(), json!("API Connection succeeded"));
}

#[tokio::test]
async fn plex_tautulli_and_overseerr_answer_for_the_right_key_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;

        let plex = homepage_services::plex_server().await;
        let settings =
            |token: &str| vec![("plexURL", json!(plex.url())), ("plexToken", json!(token))];
        succeeded(&test(database, "plex", &settings(PLEX_SERVER_TOKEN)).await);
        let wrong = test(database, "plex", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            wrong.message(),
            json!("URL and/or Token not setup correctly")
        );
        let none = test(database, "plex", &[("plexURL", json!(plex.url()))]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("URL and/or Token not setup"));

        let tautulli = homepage_services::tautulli_statistics().await;
        let settings = |key: &str| {
            vec![
                ("tautulliURL", json!(tautulli.url())),
                ("tautulliApikey", json!(key)),
            ]
        };
        succeeded(&test(database, "tautulli", &settings(TAUTULLI_KEY)).await);
        let wrong = test(database, "tautulli", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::CONFLICT);
        assert_eq!(
            wrong.message(),
            json!("Tautulli Error Occurred - Check URL or Credentials")
        );
        let none = test(database, "tautulli", &settings("")).await;
        assert_eq!(none.message(), json!("Tautulli Token is not defined"));
        let none = test(database, "tautulli", &[]).await;
        assert_eq!(none.message(), json!("Tautulli URL is not defined"));

        let overseerr = homepage_services::overseerr_requests().await;
        let settings = |key: &str| {
            vec![
                ("overseerrURL", json!(overseerr.url())),
                ("overseerrToken", json!(key)),
            ]
        };
        succeeded(&test(database, "overseerr", &settings(OVERSEERR_KEY)).await);
        let wrong = test(database, "overseerr", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
        assert_eq!(wrong.message(), json!("API Connection failed"));
        let none = test(database, "overseerr", &[]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
async fn sonarr_radarr_and_lidarr_name_the_install_that_failed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let sonarr = homepage_services::sonarr().await;
        let radarr = homepage_services::radarr().await;
        let lidarr = homepage_services::lidarr().await;
        let cases = [
            ("sonarr", "Sonarr", "sonarrURL", "sonarrToken", sonarr.url()),
            ("radarr", "Radarr", "radarrURL", "radarrToken", radarr.url()),
            ("lidarr", "Lidarr", "lidarrURL", "lidarrToken", lidarr.url()),
        ];
        for (service, name, url_key, token_key, url) in cases {
            let good = [(url_key, json!(url)), (token_key, json!(ARR_KEY))];
            let answer = test(database, service, &good).await;
            assert_eq!(answer.status, StatusCode::OK, "{service}: {}", answer.body);
            assert_eq!(answer.message(), Value::Null);

            // Two installs: the second one has a wrong key, and is named.
            let two = [
                (url_key, json!(format!("{url},{url}/"))),
                (token_key, json!(format!("{ARR_KEY},wrong"))),
            ];
            let answer = test(database, service, &two).await;
            assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(answer.message(), json!(format!("{url}: Unauthorized")));

            let no_url = test(database, service, &[(token_key, json!(ARR_KEY))]).await;
            assert_eq!(no_url.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                no_url.message(),
                json!(format!("{name} URL is not defined"))
            );
            let no_token = test(database, service, &[(url_key, json!(url))]).await;
            assert_eq!(
                no_token.message(),
                json!(format!("{name} Token is not defined"))
            );
        }

        // An install that cannot be reached is named, with the fault.
        let down = [
            ("sonarrURL", json!("http://127.0.0.1:9")),
            ("sonarrToken", json!(ARR_KEY)),
        ];
        let answer = test(database, "sonarr", &down).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        let message = answer.message().as_str().unwrap_or_default().to_owned();
        assert!(message.starts_with("http://127.0.0.1:9: "), "{message}");
    })
    .await;
}

#[tokio::test]
async fn download_clients_answer_for_the_right_login_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;

        let sabnzbd = homepage_services::sabnzbd().await;
        let settings = |key: &str| {
            vec![
                ("sabnzbdURL", json!(sabnzbd.url())),
                ("sabnzbdToken", json!(key)),
            ]
        };
        let good = test(database, "sabnzbd", &settings(SABNZBD_KEY)).await;
        succeeded(&good);
        assert!(good.data()["queue"].is_object(), "{}", good.body);
        let wrong = test(database, "sabnzbd", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!wrong.body.contains("wrong"), "{}", wrong.body);
        let none = test(database, "sabnzbd", &[]).await;
        assert_eq!(none.message(), json!("URL and/or Token not setup"));

        let qbittorrent = homepage_services::qbittorrent().await;
        let settings = |password: &str| {
            vec![
                ("qBittorrentURL", json!(qbittorrent.url())),
                ("qBittorrentUsername", json!(QBITTORRENT_LOGIN.0)),
                ("qBittorrentPassword", json!(password)),
                ("qBittorrentApiVersion", json!("2")),
            ]
        };
        succeeded(&test(database, "qbittorrent", &settings(QBITTORRENT_LOGIN.1)).await);
        let wrong = test(database, "qbittorrent", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::CONFLICT);
        let none = test(database, "qbittorrent", &[]).await;
        assert_eq!(none.message(), json!("qBittorrent URL is not defined"));

        let nzbget = homepage_services::nzbget().await;
        let settings = |password: &str| {
            vec![
                ("nzbgetURL", json!(nzbget.url())),
                ("nzbgetUsername", json!(NZBGET_LOGIN.0)),
                ("nzbgetPassword", json!(password)),
            ]
        };
        succeeded(&test(database, "nzbget", &settings(NZBGET_LOGIN.1)).await);
        let wrong = test(database, "nzbget", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("NZBGet: An Error Occurred"));
        assert_eq!(wrong.json()["response"]["result"], json!("error"));
        let none = test(database, "nzbget", &[]).await;
        assert_eq!(none.message(), json!("NZBGet URL is not defined"));

        let transmission = homepage_services::transmission().await;
        let settings = |password: &str| {
            vec![
                ("transmissionURL", json!(transmission.url())),
                ("transmissionUsername", json!(TRANSMISSION_LOGIN.0)),
                ("transmissionPassword", json!(password)),
            ]
        };
        succeeded(&test(database, "transmission", &settings(TRANSMISSION_LOGIN.1)).await);
        let wrong = test(database, "transmission", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            wrong.message(),
            json!("Transmission Connect Function - Error: Could not get session ID")
        );
        let none = test(database, "transmission", &[]).await;
        assert_eq!(none.message(), json!("Transmission URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn pihole_names_the_host_that_failed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let pihole = homepage_services::pihole().await;
        let settings = |token: &str| {
            vec![
                ("piholeURL", json!(pihole.url())),
                ("piholeToken", json!(token)),
            ]
        };
        let good = test(database, "pihole", &settings(PIHOLE_TOKEN)).await;
        assert_eq!(good.status, StatusCode::OK, "{}", good.body);
        assert_eq!(good.message(), Value::Null);
        assert_eq!(pihole.seen()[0].path, "/api/auth");
        assert_eq!(pihole.seen()[0].json(), json!({"password": PIHOLE_TOKEN}));
        let wrong = test(database, "pihole", &settings("wrong")).await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("127.0.0.1: Login failed"));
        let missing = test(database, "pihole", &settings("")).await;
        assert_eq!(missing.message(), json!("127.0.0.1: Missing API Token"));
        let none = test(database, "pihole", &[]).await;
        assert_eq!(none.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(none.message(), json!("Pihole URL is not defined"));
    })
    .await;
}

#[tokio::test]
async fn only_an_admin_or_a_co_admin_runs_a_test() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let app = installed_app_with(database, &[]);
        let boss = seed::logged_in(database, &app, "pat", 1).await;
        let user = seed::logged_in(database, &app, "robin", 4).await;
        let services = [
            "plex",
            "sonarr",
            "radarr",
            "lidarr",
            "sabnzbd",
            "qbittorrent",
            "tautulli",
            "overseerr",
            "nzbget",
            "transmission",
            "pihole",
        ];
        for service in services {
            let path = format!("/test/{service}");
            for who in [As::Guest, As::User(&user)] {
                let answer = call(&app, who, "POST", &path, None).await;
                assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{service}");
            }
            // Nothing is set up, so the test of a Co-Admin says what is missing.
            let answer = call(&app, As::User(&boss), "POST", &path, None).await;
            assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY, "{service}");
        }
    })
    .await;
}
