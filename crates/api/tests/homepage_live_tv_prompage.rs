//! The Emby live television item with its connection test, and the PromPage item,
//! against stand-ins for an Emby server and for PromPage.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, MEDIA_SERVER_KEY};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const STATS: &str = "/homepage/embyLiveTVTracker/stats";
const ACTIVITY: &str = "/homepage/embyLiveTVTracker/activity";
const TEST: &str = "/test/embyLiveTVTracker";
const PROMPAGE: &str = "/homepage/prompage/data";

type Changed = Vec<(&'static str, Value)>;

fn live_tv(url: &str) -> Changed {
    vec![
        ("homepageEmbyLiveTVTrackerEnabled", json!(true)),
        ("homepageEmbyLiveTVTrackerAuth", json!("4")),
        ("embyURL", json!(url)),
        ("embyToken", json!(MEDIA_SERVER_KEY)),
    ]
}

/// The settings with another value for one of them.
fn with(mut settings: Changed, key: &'static str, value: Value) -> Changed {
    settings.retain(|(known, _)| *known != key);
    settings.push((key, value));
    settings
}

#[tokio::test]
async fn live_television_has_its_numbers_and_its_lists() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let emby = homepage_services::emby_live_tv().await;
        let app = installed_app_with(database, &live_tv(&emby.url()));

        let answer = call(&app, As::Admin, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("LiveTV stats retrieved successfully"));
        let stats = answer.data();
        assert_eq!(stats["activeTimers"], 2);
        assert_eq!(stats["seriesTimers"], 1);
        assert_eq!(stats["todaysRecordings"], 0);
        assert_eq!(stats["totalRecordings"], 31);
        assert_eq!(stats["recentRecordings"].as_array().map(Vec::len), Some(2));
        assert_eq!(stats["recentRecordings"][0]["program"], "The Film");
        assert_eq!(stats["recentRecordings"][0]["channel"], "Two");
        assert_eq!(stats["recentRecordings"][1]["series"], "The News");
        // The key goes in a header.
        let seen = emby.seen();
        assert_eq!(seen[0].path, "/emby/LiveTv/Timers");
        assert_eq!(seen[0].header("x-emby-token"), Some(MEDIA_SERVER_KEY));
        assert!(seen.iter().all(|request| !request.path.contains(MEDIA_SERVER_KEY)));

        let answer = call(&app, As::Admin, "GET", ACTIVITY, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.message(), json!("LiveTV activity retrieved successfully"));
        assert_eq!(
            answer.data()["scheduledRecordings"],
            json!([
                {
                    "date": "2001-03-04T20:00:00.0000000Z", "name": "News", "seriesName": "The News",
                    "episodeTitle": "", "channelName": "One", "userName": null, "duration": "30m",
                    "status": "Completed", "type": "timer",
                },
                {
                    "date": "2099-01-05T22:00:00.0000000Z", "name": "Guests",
                    "seriesName": "The Late Show", "episodeTitle": "Guests",
                    "channelName": "Channel 12", "userName": null, "duration": "1h 30m",
                    "status": "Scheduled", "type": "timer",
                },
            ])
        );
        assert_eq!(
            answer.data()["completedRecordings"],
            json!([
                {
                    "date": "2026-11-18T20:15:00.0000000Z", "name": "The Film", "seriesName": "",
                    "channelName": "Unknown Channel", "userName": null, "duration": "1h 30m",
                    "status": "Completed", "type": "recording",
                },
                {
                    "date": "2026-11-17T06:00:00.0000000Z", "name": "News", "seriesName": "The News",
                    "channelName": "Unknown Channel", "userName": null, "duration": "-",
                    "status": "Completed", "type": "recording",
                },
            ])
        );

        // With the names of the users, without the recordings, and with one timer.
        let mut other = live_tv(&emby.url());
        other.extend([
            ("homepageEmbyLiveTVTrackerShowUserInfo", json!(true)),
            ("homepageEmbyLiveTVTrackerShowCompleted", json!(false)),
        ]);
        let named = installed_app_with(database, &other);
        let answer = call(&named, As::Admin, "GET", ACTIVITY, None).await;
        assert_eq!(answer.data()["scheduledRecordings"][1]["userName"], "Zed");
        assert_eq!(answer.data()["completedRecordings"], json!([]));
        let one = with(other, "homepageEmbyLiveTVTrackerMaxItems", json!("1"));
        let answer = call(&installed_app_with(database, &one), As::Admin, "GET", ACTIVITY, None).await;
        assert_eq!(answer.data()["scheduledRecordings"].as_array().map(Vec::len), Some(1));

        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        assert_eq!(
            items.data(),
            json!([{
                "key": "homepageOrderEmbyLiveTVTracker",
                "kind": "embylivetv",
                "id": "",
                "options": {
                    "refreshMinutes": 5, "compact": false, "showDuration": true,
                    "showSeriesInfo": true, "showUserInfo": false, "maxItems": 10,
                    "showCompleted": true, "maxCompletedItems": 5,
                },
            }])
        );
    })
    .await;
}

#[tokio::test]
async fn live_television_is_refused_and_fails_as_in_the_php_application() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let emby = homepage_services::emby_live_tv().await;
        let url = emby.url();

        let off = with(
            live_tv(&url),
            "homepageEmbyLiveTVTrackerEnabled",
            json!(false),
        );
        let off = installed_app_with(database, &off);
        let answer = call(&off, As::Admin, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepageEmbyLiveTVTrackerEnabled module is not enabled")
        );
        // The list has one answer for every reason.
        let answer = call(&off, As::Admin, "GET", ACTIVITY, None).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        assert_eq!(answer.message(), json!("Permission denied"));

        let on = installed_app_with(database, &live_tv(&url));
        let answer = call(&on, As::Guest, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            answer.message(),
            json!("User not approved to view this homepage item")
        );
        let answer = call(&on, As::Guest, "GET", ACTIVITY, None).await;
        assert_eq!(answer.status, StatusCode::FORBIDDEN);
        let user = seed::logged_in(database, &on, "uma", 4).await;
        let answer = call(&on, As::User(&user), "GET", ACTIVITY, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);

        let no_key = installed_app_with(database, &with(live_tv(&url), "embyToken", json!("")));
        let answer = call(&no_key, As::Admin, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("embyToken was not supplied"));

        // A server that refuses the key, or that is not there, leaves the numbers at 0.
        let nothing = json!({
            "activeTimers": 0, "seriesTimers": 0, "todaysRecordings": 0,
            "totalRecordings": 0, "recentRecordings": [],
        });
        let wrong = installed_app_with(database, &with(live_tv(&url), "embyToken", json!("wrong")));
        let answer = call(&wrong, As::Admin, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), nothing);
        let away = installed_app_with(database, &live_tv("http://127.0.0.1:1"));
        let answer = call(&away, As::Admin, "GET", STATS, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(answer.data(), nothing);
        let answer = call(&away, As::Admin, "GET", ACTIVITY, None).await;
        assert_eq!(
            answer.data(),
            json!({"scheduledRecordings": [], "completedRecordings": []})
        );
        assert!(!answer.body.contains(MEDIA_SERVER_KEY));
    })
    .await;
}

#[tokio::test]
async fn test_of_the_live_television_names_the_server() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let emby = homepage_services::emby_live_tv().await;
        let test = async |changed: &Changed, who: As<'_>| {
            call(
                &installed_app_with(database, changed),
                who,
                "POST",
                TEST,
                None,
            )
            .await
        };

        let answer = test(&live_tv(&emby.url()), As::Admin).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("Successfully connected to Made-up Emby with LiveTV support enabled")
        );
        assert_eq!(emby.seen()[0].path, "/emby/System/Info");
        assert_eq!(emby.seen()[1].path, "/emby/LiveTv/Info");

        let wrong = test(
            &with(live_tv(&emby.url()), "embyToken", json!("wrong")),
            As::Admin,
        )
        .await;
        assert_eq!(wrong.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(wrong.message(), json!("Emby Connection Error"));

        let away = test(&live_tv("http://127.0.0.1:1"), As::Admin).await;
        assert_eq!(away.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!away.body.contains(MEDIA_SERVER_KEY), "{}", away.body);
        assert!(!away.body.contains("127.0.0.1"), "{}", away.body);

        let off = with(
            live_tv(&emby.url()),
            "homepageEmbyLiveTVTrackerEnabled",
            json!(false),
        );
        let answer = test(&off, As::Admin).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            test(&live_tv(&emby.url()), As::Guest).await.status,
            StatusCode::UNAUTHORIZED
        );
    })
    .await;
}

#[tokio::test]
async fn prompage_lists_its_services_for_everybody() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let service = homepage_services::prompage().await;
        let settings = |url: &str| -> Changed {
            vec![
                ("homepagePromPageEnabled", json!(true)),
                ("promPageURL", json!(url)),
                ("homepagePromPageHeader", json!("Our services")),
                ("homepagePromPageCompact", json!(false)),
            ]
        };
        let app = installed_app_with(database, &settings(&format!("{}/", service.url())));

        // The item has no level in the PHP application: a guest gets it too.
        let answer = call(&app, As::Guest, "GET", PROMPAGE, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!({
                "data": [
                    {"name": "Blog", "group": "Web", "status": true, "uptime": 99.954},
                    {"name": "Mail", "group": "Web", "status": false, "uptime": 87.5},
                ],
                "options": {
                    "title": "Our services", "titleToggle": true, "compact": false,
                    "showUptime": true,
                },
            })
        );
        assert_eq!(service.seen()[0].path, "/api/services");
        let items = call(&app, As::Guest, "GET", "/homepage/items", None).await;
        assert_eq!(
            items.data(),
            json!([{"key": "homepageOrderPromPage", "kind": "prompage", "id": ""}])
        );

        let off = with(
            settings(&service.url()),
            "homepagePromPageEnabled",
            json!(false),
        );
        let answer = call(
            &installed_app_with(database, &off),
            As::Admin,
            "GET",
            PROMPAGE,
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepagePromPageEnabled module is not enabled")
        );
        let answer = call(
            &installed_app_with(database, &settings("")),
            As::Admin,
            "GET",
            PROMPAGE,
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(answer.message(), json!("promPageURL was not supplied"));

        // A page that is not PromPage, and a server that is not there.
        let pages = homepage_services::pages().await;
        let other = installed_app_with(database, &settings(&pages.url()));
        let answer = call(&other, As::Admin, "GET", PROMPAGE, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert_eq!(
            answer.message(),
            json!("PromPage answered with the status 404")
        );
        let away = installed_app_with(database, &settings("http://127.0.0.1:1"));
        let answer = call(&away, As::Admin, "GET", PROMPAGE, None).await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED);
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);
    })
    .await;
}
