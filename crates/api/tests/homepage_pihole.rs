//! The Pi-hole item of the homepage, against stand-ins for Pi-hole.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, PIHOLE_SID, PIHOLE_TOKEN};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const PATH: &str = "/homepage/pihole/stats";

#[tokio::test]
async fn statistics_of_each_install_come_by_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let first = homepage_services::pihole().await;
        let second = homepage_services::pihole().await;
        let settings = |passwords: &str| {
            vec![
                ("homepagePiholeEnabled", json!(true)),
                (
                    "piholeURL",
                    json!(format!("{}/,{}", first.url(), second.url())),
                ),
                ("piholeToken", json!(passwords)),
                ("homepagePiholeCombine", json!(true)),
            ]
        };

        let both = format!("{PIHOLE_TOKEN},{PIHOLE_TOKEN}");
        let app = installed_app_with(database, &settings(&both));
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        let one = &data["data"][first.url()];
        assert_eq!(one["sum_queries"], json!(12000));
        assert_eq!(one["sum_blocked"], json!(3000));
        assert_eq!(one["percent_blocked"], json!(25));
        assert_eq!(
            one["domains_being_blocked"],
            json!(["ads.example.com", "tracker.example.net"])
        );
        assert_eq!(data["data"][second.url()]["sum_queries"], json!(12000));
        assert_eq!(data["options"], json!({"combine": true, "title": true}));

        // The login with the password, then the older session of Organizr is removed,
        // and then the statistics of the last 24 hours are asked with the session.
        let seen = first.seen();
        let steps: Vec<(&str, &str)> = seen
            .iter()
            .map(|seen| {
                (
                    seen.method.as_str(),
                    seen.path.split('?').next().unwrap_or_default(),
                )
            })
            .collect();
        assert_eq!(
            steps,
            [
                ("POST", "/api/auth"),
                ("GET", "/api/auth/sessions"),
                ("DELETE", "/api/auth/session/1"),
                ("GET", "/api/stats/database/summary"),
                ("GET", "/api/stats/database/top_domains"),
            ]
        );
        assert_eq!(seen[0].json(), json!({"password": PIHOLE_TOKEN}));
        assert_eq!(seen[0].header("user-agent"), Some("Organizr"));
        assert_eq!(seen[3].header("sid"), Some(PIHOLE_SID));
        let range = |path: &str| -> (i64, i64) {
            let field = |name: &str| {
                path.split(['?', '&'])
                    .find_map(|part| part.strip_prefix(&format!("{name}=")))
                    .and_then(|value| value.parse::<i64>().ok())
                    .expect("the field is a number")
            };
            (field("from"), field("until"))
        };
        let (from, until) = range(&seen[3].path);
        assert_eq!(until - from, 86_400);
        assert!(
            seen[4].path.contains("&blocked=true&count=1000"),
            "{}",
            seen[4].path
        );

        // A password that Pi-hole refuses gives only an empty list of blocked domains,
        // as in PHP, and no further request.
        let app = installed_app_with(database, &settings("wrong,wrong"));
        let before = first.seen().len();
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data()["data"][first.url()],
            json!({"domains_being_blocked": []})
        );
        assert_eq!(first.seen().len(), before + 1);

        // An install without a password is an error that names its host.
        let app = installed_app_with(database, &settings(""));
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(answer.message(), json!("127.0.0.1: Missing API Token"));
    })
    .await;
}

#[tokio::test]
async fn install_that_cannot_be_reached_gives_an_error_without_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings: Vec<(&str, Value)> = vec![
            ("homepagePiholeEnabled", json!(true)),
            ("piholeURL", json!("http://127.0.0.1:9")),
            ("piholeToken", json!("made-up-secret-token")),
        ];
        let app = installed_app_with(database, &settings);
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !answer.body.contains("made-up-secret-token"),
            "{}",
            answer.body
        );
        assert!(!answer.body.contains("127.0.0.1"), "{}", answer.body);

        // The item has the checks of every item.
        let guest = call(&app, As::Guest, "GET", PATH, None).await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED);
        let off = installed_app_with(database, &settings[1..]);
        let answer = call(&off, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::CONFLICT);
        assert_eq!(
            answer.message(),
            json!("homepagePiholeEnabled module is not enabled")
        );
        let items = call(&app, As::Admin, "GET", "/homepage/items", None).await;
        let kinds: Vec<&str> = items
            .data()
            .as_array()
            .expect("the items are a list")
            .iter()
            .filter_map(|item| item["kind"].as_str())
            .collect::<Vec<_>>()
            .into_iter()
            .map(|kind| if kind == "pihole" { "pihole" } else { "other" })
            .collect();
        assert!(kinds.contains(&"pihole"), "{}", items.body);
    })
    .await;
}
