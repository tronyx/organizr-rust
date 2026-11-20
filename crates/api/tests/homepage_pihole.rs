//! The Pi-hole item of the homepage, against stand-ins for Pi-hole.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::http::StatusCode;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::homepage_services::{self, PIHOLE_TOKEN};
use organizr_testkit::http::{As, call};
use organizr_testkit::{installed_app_with, seed};
use serde_json::{Value, json};

const PATH: &str = "/homepage/pihole/stats";

#[tokio::test]
async fn summary_of_each_install_comes_by_its_host() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let first = homepage_services::pihole().await;
        let second = homepage_services::pihole().await;
        // Both stand-ins are on this machine. The second one is named by its host name,
        // so that the two have different hosts.
        let second_url = second.url().replace("127.0.0.1", "localhost");
        assert_ne!(second_url, second.url());
        let settings = |tokens: &str| {
            vec![
                ("homepagePiholeEnabled", json!(true)),
                (
                    "piholeURL",
                    json!(format!("{}/admin/,{second_url}/admin", first.url())),
                ),
                ("piholeToken", json!(tokens)),
                ("homepagePiholeCombine", json!(true)),
            ]
        };

        let both = format!("{PIHOLE_TOKEN},{PIHOLE_TOKEN}");
        let app = installed_app_with(database, &settings(&both));
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["data"]["127.0.0.1"]["dns_queries_today"], json!(12000));
        assert_eq!(data["data"]["localhost"]["ads_blocked_today"], json!(3000));
        assert_eq!(data["options"], json!({"combine": true, "title": true}));
        let seen = first.seen();
        assert_eq!(
            seen[0].path,
            format!("/admin/api.php?summaryRaw&auth={PIHOLE_TOKEN}")
        );

        // An address without a token at its place is not asked. A wrong token gets the
        // empty list that Pi-hole sends for it.
        let app = installed_app_with(database, &settings("wrong"));
        let before = second.seen().len();
        let answer = call(&app, As::Admin, "GET", PATH, None).await;
        assert_eq!(answer.data()["data"], json!({"127.0.0.1": []}));
        assert_eq!(second.seen().len(), before);

        // Without a token, the request has none.
        let app = installed_app_with(database, &settings(""));
        call(&app, As::Admin, "GET", PATH, None).await;
        let seen = first.seen();
        assert_eq!(
            seen.last().expect("a request came").path,
            "/admin/api.php?summaryRaw"
        );
    })
    .await;
}

#[tokio::test]
async fn install_that_cannot_be_reached_gives_an_error_without_its_address() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let settings: Vec<(&str, Value)> = vec![
            ("homepagePiholeEnabled", json!(true)),
            ("piholeURL", json!("http://127.0.0.1:9/admin")),
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
