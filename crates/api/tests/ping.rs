//! `POST /ping` and `GET /ping/{host}` through the real route table: what the recorded
//! PHP answers cannot show.
//!
//! The recording holds only targets that do not answer. These tests use a stand-in for
//! the network, and look at which targets the server probes for which caller (D-08).

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::http::StatusCode;
use organizr_api::ping::{PingTarget, Prober};
use organizr_api::router;
use organizr_db::Database;
use organizr_settings::Settings;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{Answer, As, call};
use organizr_testkit::{seed, state_with};
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// The targets that a stand-in for the network was asked to probe.
type Probed = Arc<Mutex<Vec<PingTarget>>>;

/// The settings of an install that is set up, with other values for some of them.
fn settings(changed: &[(&str, Value)]) -> Settings {
    let mut settings = Settings::defaults();
    let installed = [
        ("uuid", json!(seed::INSTALL_ID)),
        ("organizrAPI", json!(seed::API_KEY)),
    ];
    for (key, value) in installed.iter().chain(changed) {
        settings
            .set(key, value)
            .unwrap_or_else(|error| panic!("the setting `{key}` cannot be set: {error}"));
    }
    settings
}

/// The application with a stand-in for the network: every target answers after 12.5
/// milliseconds, but the host `down.example.com`.
fn app_with_stand_in(database: &Database, changed: &[(&str, Value)]) -> (Router, Probed) {
    let probed = Probed::default();
    let seen = Arc::clone(&probed);
    let stand_in = Prober::new(move |target: PingTarget| {
        let answers = target.host != "down.example.com";
        seen.lock().expect("the list can be locked").push(target);
        async move { answers.then_some(12.5) }
    });
    let state = state_with(database, settings(changed)).with_prober(stand_in);
    (router(state).expect("the route table is valid"), probed)
}

/// Adds a tab with an address to check. `least_access` is the level of the last group
/// that sees the tab.
async fn add_tab(app: &Router, name: &str, ping_url: &str, ping: u8, least_access: u16) {
    let body = json!({
        "name": name,
        "url": format!("https://{}.example.com/", name.to_lowercase()),
        "image": "fontawesome::cog",
        "enabled": 1,
        "group_id": least_access,
        "ping": ping,
        "ping_url": ping_url,
    });
    let answer = call(app, As::Admin, "POST", "/tabs", Some(body)).await;
    assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
}

async fn ping_list(app: &Router, who: As<'_>, list: Value) -> Answer {
    call(app, who, "POST", "/ping", Some(json!({ "list": list }))).await
}

fn targets(probed: &Probed) -> Vec<(String, Option<u16>)> {
    probed
        .lock()
        .expect("the list can be locked")
        .iter()
        .map(|target| (target.host.clone(), target.port))
        .collect()
}

#[tokio::test]
async fn only_the_address_to_check_of_a_tab_is_probed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, probed) = app_with_stand_in(database, &[]);
        add_tab(&app, "Plex", "plex.example.com:32400", 1, 1).await;
        add_tab(&app, "Down", "down.example.com", 1, 1).await;
        // The check of this tab is switched off.
        add_tab(&app, "Off", "off.example.com:80", 0, 1).await;

        let list = json!([
            "plex.example.com:32400",
            "down.example.com",
            "off.example.com:80",
            // The address of the tab itself, another port, and a host of the local network.
            "https://plex.example.com/",
            "plex.example.com:22",
            "192.168.1.1:80",
        ]);
        let answer = ping_list(&app, As::Admin, list).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!({
                "plex.example.com:32400": 12.5,
                "down.example.com": false,
                "off.example.com:80": false,
                "https://plex.example.com/": false,
                "plex.example.com:22": false,
                "192.168.1.1:80": false,
            })
        );
        let mut seen = targets(&probed);
        seen.sort();
        assert_eq!(
            seen,
            [
                ("down.example.com".to_owned(), None),
                ("plex.example.com".to_owned(), Some(32400)),
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn entry_named_many_times_is_probed_once() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, probed) = app_with_stand_in(database, &[]);
        add_tab(&app, "Plex", "plex.example.com:32400", 1, 1).await;

        let mut list = vec![json!("plex.example.com:32400"); 5_000];
        list.push(json!(" plex.example.com:32400 "));
        list.extend(vec![json!("other.example.com:80"); 5_000]);
        let answer = ping_list(&app, As::Admin, Value::Array(list)).await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!({
                "plex.example.com:32400": 12.5,
                " plex.example.com:32400 ": 12.5,
                "other.example.com:80": false,
            })
        );
        assert_eq!(
            targets(&probed),
            [("plex.example.com".to_owned(), Some(32400))]
        );
    })
    .await;
}

#[tokio::test]
async fn one_address_gets_its_time_or_nothing() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, probed) = app_with_stand_in(database, &[]);
        add_tab(&app, "Plex", "  plex.example.com:32400 ", 1, 1).await;
        add_tab(&app, "Down", "down.example.com", 1, 1).await;

        let up = ping_list(&app, As::Admin, json!("plex.example.com:32400")).await;
        assert_eq!(up.data(), json!(12.5));
        let up = call(&app, As::Admin, "GET", "/ping/plex.example.com:32400", None).await;
        assert_eq!(up.status, StatusCode::OK, "{}", up.body);
        assert_eq!(up.data(), json!(12.5));
        let down = call(&app, As::Admin, "GET", "/ping/down.example.com", None).await;
        assert_eq!(down.data(), Value::Null);
        let other = call(&app, As::Admin, "GET", "/ping/other.example.com", None).await;
        assert_eq!(other.status, StatusCode::OK, "{}", other.body);
        assert_eq!(other.data(), Value::Null);

        assert_eq!(
            targets(&probed).len(),
            3,
            "the host of no tab is not probed"
        );
    })
    .await;
}

#[tokio::test]
async fn caller_gets_only_the_tabs_of_the_own_level_probed() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        // Everybody may ask for a ping.
        let (app, probed) = app_with_stand_in(database, &[("pingAuth", json!("999"))]);
        add_tab(&app, "Open", "open.example.com:80", 1, 999).await;
        add_tab(&app, "Closed", "closed.example.com:80", 1, 1).await;
        let list = json!(["open.example.com:80", "closed.example.com:80"]);

        let for_guest = ping_list(&app, As::Guest, list.clone()).await;
        assert_eq!(
            for_guest.data(),
            json!({"open.example.com:80": 12.5, "closed.example.com:80": false})
        );
        assert_eq!(
            targets(&probed),
            [("open.example.com".to_owned(), Some(80))]
        );

        let for_admin = ping_list(&app, As::Admin, list).await;
        assert_eq!(
            for_admin.data(),
            json!({"open.example.com:80": 12.5, "closed.example.com:80": 12.5})
        );
    })
    .await;
}

#[tokio::test]
async fn setting_decides_who_may_ask_for_a_ping() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, probed) = app_with_stand_in(database, &[]);
        add_tab(&app, "Open", "open.example.com:80", 1, 999).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;
        let super_user = seed::logged_in(database, &app, "sam", 2).await;

        // The level of a new install is the one of a Co-Admin.
        for who in [As::Guest, As::User(&super_user)] {
            let answer = ping_list(&app, who, json!(["open.example.com:80"])).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.body);
            assert_eq!(answer.message(), json!("Not Authorized"));
            let answer = call(&app, who, "GET", "/ping/open.example.com:80", None).await;
            assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.body);
        }
        assert_eq!(targets(&probed), []);

        let answer = ping_list(&app, As::User(&co_admin), json!(["open.example.com:80"])).await;
        assert_eq!(answer.data(), json!({"open.example.com:80": 12.5}));
    })
    .await;
}

#[tokio::test]
async fn list_must_hold_text_only() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let (app, probed) = app_with_stand_in(database, &[]);
        add_tab(&app, "Plex", "plex.example.com:32400", 1, 1).await;

        for list in [
            json!(["plex.example.com:32400", 5]),
            json!([["plex.example.com:32400"]]),
            json!({"host": "plex.example.com:32400"}),
            json!(5),
            json!(true),
        ] {
            let answer = ping_list(&app, As::Admin, list.clone()).await;
            assert_eq!(
                answer.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{list}: {}",
                answer.body
            );
            assert_eq!(answer.message(), json!("list is not valid"));
        }
        assert_eq!(targets(&probed), []);
    })
    .await;
}

#[tokio::test]
async fn port_that_listens_answers_over_the_real_network() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port can be opened");
        let address = listener
            .local_addr()
            .expect("the port has an address")
            .to_string();
        // This application probes with the network, as the server does.
        let state = state_with(database, settings(&[]));
        let app = router(state).expect("the route table is valid");
        add_tab(&app, "Local", &address, 1, 1).await;

        let up = ping_list(&app, As::Admin, json!([address])).await;
        let time = &up.data()[&address];
        assert!(time.as_f64().is_some_and(|time| time >= 0.0), "{}", up.body);

        drop(listener);
        let down = ping_list(&app, As::Admin, json!([address])).await;
        assert_eq!(down.data()[&address], json!(false));
    })
    .await;
}
