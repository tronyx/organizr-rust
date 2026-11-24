//! The Plex Libraries plugin: who sees which shares of the Plex server, and who may change
//! which libraries of a share (D-121). Against a stand-in for `plex.tv`.

#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test reads the answers it knows, and its helpers may panic"
)]

use axum::Router;
use axum::http::StatusCode;
use organizr_api::router;
use organizr_db::Database;
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{As, call};
use organizr_testkit::services::{self, OWNER_TOKEN, SERVER_ID, StandIn};
use organizr_testkit::{NO_LOGIN_LIMIT, installed_settings, seed, state_with};
use serde_json::{Value, json};

fn app_with(database: &Database, plex: &StandIn, more: &[(&'static str, Value)]) -> Router {
    let mut settings = vec![
        ("PLEXLIBRARIES-enabled", json!(true)),
        ("plexToken", json!(OWNER_TOKEN)),
        ("plexID", json!(SERVER_ID)),
        ("loginAttempts", json!(NO_LOGIN_LIMIT)),
    ];
    settings.extend(more.iter().cloned());
    let state = state_with(database, installed_settings(&settings)).with_plex_tv(&plex.url());
    router(state).expect("the route table is valid")
}

/// The ids of the libraries of a share in an answer that are shared, and those that can
/// be changed.
fn libraries(share: &Value) -> (Vec<String>, Vec<String>) {
    let list = share["libraries"].as_array().expect("libraries");
    let ids = |pick: &str| {
        list.iter()
            .filter(|library| library[pick] == json!(true))
            .map(|library| library["id"].as_str().expect("an id").to_owned())
            .collect()
    };
    (ids("shared"), ids("changeable"))
}

#[tokio::test]
async fn admin_sees_every_share_and_changes_any_library() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(database, &plex, &[]);

        let answer = call(
            &app,
            As::Admin,
            "GET",
            "/plugins/plexlibraries/shares",
            None,
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["manager"], json!(true));
        let shares = data["shares"].as_array().expect("shares");
        assert_eq!(shares.len(), 2);
        assert_eq!(shares[0]["username"], json!("Robin"));
        assert_eq!(
            libraries(&shares[0]),
            (
                vec!["7001".to_owned()],
                vec!["7001".to_owned(), "7002".to_owned(), "7003".to_owned()]
            )
        );
        // The token of a share never reaches the browser.
        assert!(!answer.body.contains("secret-of"));

        let body = json!({"libraries": ["7001", "7003"]});
        let changed = call(
            &app,
            As::Admin,
            "PUT",
            "/plugins/plexlibraries/shares/9001",
            Some(body),
        )
        .await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let sent = plex.seen().pop().expect("plex.tv was asked").json();
        assert_eq!(
            sent["shared_server"]["library_section_ids"],
            json!([7001, 7003])
        );
    })
    .await;
}

#[tokio::test]
async fn user_sees_the_own_share_and_changes_only_the_libraries_of_the_setting() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        // Users may switch Shows; Films and Music stay as the Admin set them.
        let app = app_with(
            database,
            &plex,
            &[("PLEXLIBRARIES-librariesToInclude", json!("7002"))],
        );
        let robin = seed::logged_in(database, &app, "robin", 4).await;

        let answer = call(
            &app,
            As::User(&robin),
            "GET",
            "/plugins/plexlibraries/shares",
            None,
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        let data = answer.data();
        assert_eq!(data["manager"], json!(false));
        let shares = data["shares"].as_array().expect("shares");
        assert_eq!(shares.len(), 1, "only the own share");
        assert_eq!(shares[0]["id"], json!("9001"));
        assert_eq!(
            libraries(&shares[0]),
            (vec!["7001".to_owned()], vec!["7002".to_owned()])
        );

        // Robin switches Shows on and tries to drop Films and add Music: only Shows counts.
        let body = json!({"libraries": ["7002", "7003"]});
        let changed = call(
            &app,
            As::User(&robin),
            "PUT",
            "/plugins/plexlibraries/shares/9001",
            Some(body),
        )
        .await;
        assert_eq!(changed.status, StatusCode::OK, "{}", changed.body);
        let sent = plex.seen().pop().expect("plex.tv was asked").json();
        assert_eq!(
            sent["shared_server"]["library_section_ids"],
            json!([7001, 7002])
        );

        // The share of another person is not robin's to change.
        let other = call(
            &app,
            As::User(&robin),
            "PUT",
            "/plugins/plexlibraries/shares/9002",
            Some(json!({"libraries": []})),
        )
        .await;
        assert_eq!(other.status, StatusCode::UNAUTHORIZED, "{}", other.body);
    })
    .await;
}

#[tokio::test]
async fn plugin_needs_its_switch_and_its_level() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let off = app_with(database, &plex, &[("PLEXLIBRARIES-enabled", json!(false))]);
        let answer = call(
            &off,
            As::Admin,
            "GET",
            "/plugins/plexlibraries/shares",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::CONFLICT, "{}", answer.body);
        assert_eq!(
            answer.message(),
            json!("PLEXLIBRARIES-enabled module is not enabled")
        );

        let strict = app_with(
            database,
            &plex,
            &[("PLEXLIBRARIES-Auth-include", json!("1"))],
        );
        let robin = seed::logged_in(database, &strict, "robin", 4).await;
        let answer = call(
            &strict,
            As::User(&robin),
            "GET",
            "/plugins/plexlibraries/shares",
            None,
        )
        .await;
        assert_eq!(answer.status, StatusCode::UNAUTHORIZED, "{}", answer.body);
        let guest = call(
            &strict,
            As::Guest,
            "GET",
            "/plugins/plexlibraries/shares",
            None,
        )
        .await;
        assert_eq!(guest.status, StatusCode::UNAUTHORIZED, "{}", guest.body);
    })
    .await;
}

#[tokio::test]
async fn manager_makes_the_tab_of_the_page_once() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(database, &plex, &[]);

        let made = call(
            &app,
            As::Admin,
            "GET",
            "/plugins/plexlibraries/setup/tab",
            None,
        )
        .await;
        assert_eq!(made.status, StatusCode::OK, "{}", made.body);
        assert_eq!(made.message(), json!("Tab created"));
        let again = call(
            &app,
            As::Admin,
            "GET",
            "/plugins/plexlibraries/setup/tab",
            None,
        )
        .await;
        assert_eq!(again.message(), json!("Tab already exists"));

        let tab = database
            .tabs()
            .find_by_url("api/v2/plugins/plexlibraries/page")
            .await
            .expect("the lookup works")
            .expect("the tab exists");
        assert_eq!(tab.name, "Plex Libraries");
    })
    .await;
}

#[tokio::test]
async fn library_list_names_the_libraries_of_the_plex_server() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        let app = app_with(database, &plex, &[]);
        let user = seed::logged_in(database, &app, "uma", 4).await;
        let co_admin = seed::logged_in(database, &app, "cora", 1).await;

        let answer = call(
            &app,
            As::User(&co_admin),
            "GET",
            "/plugins/plexlibraries/libraries",
            None,
        )
        .await;

        assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
        assert_eq!(
            answer.data(),
            json!([
                {"label": "Films", "value": "7001"},
                {"label": "Shows", "value": "7002"},
                {"label": "Music", "value": "7003"},
            ])
        );
        for who in [As::Guest, As::User(&user)] {
            let refused = call(&app, who, "GET", "/plugins/plexlibraries/libraries", None).await;
            assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
        }
    })
    .await;
}

#[tokio::test]
async fn library_list_is_empty_without_plex_or_when_plex_fails() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let plex = services::plex_tv().await;
        for more in [
            vec![("plexToken", json!(""))],
            vec![("plexToken", json!("a-token-that-plex-does-not-know"))],
        ] {
            let app = app_with(database, &plex, &more);
            let answer = call(
                &app,
                As::Admin,
                "GET",
                "/plugins/plexlibraries/libraries",
                None,
            )
            .await;
            assert_eq!(answer.status, StatusCode::OK, "{}", answer.body);
            assert_eq!(answer.data(), json!([]));
        }
    })
    .await;
}
