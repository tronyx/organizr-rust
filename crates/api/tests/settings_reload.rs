//! The settings of a running application can change.
//!
//! The set-up wizard and the settings pages write settings while the application runs.
//! A request that starts after the change must work with the new values. The name of
//! the session cookie shows which install id a login used.

#![allow(
    clippy::expect_used,
    reason = "helper functions of a test file may panic; the lint settings only exempt test functions"
)]

use axum::Router;
use organizr_api::{AppState, router, stored_settings};
use organizr_settings::{Settings, SettingsError};
use organizr_testkit::engines::on_each_engine;
use organizr_testkit::http::{request, send};
use organizr_testkit::seed;
use serde_json::json;

const FIRST_ID: &str = "11111111-1111-4111-8111-111111111111";
const SECOND_ID: &str = "22222222-2222-4222-8222-222222222222";

/// Logs the admin in and returns the name of the session cookie that the answer sets.
async fn session_cookie_of_a_login(app: &Router) -> String {
    let body = json!({"username": seed::ADMIN_NAME, "password": seed::ADMIN_PASSWORD});
    let headers = [("content-type", "application/json")];
    let answer = send(
        app,
        request("POST", "/api/v2/login", &headers, &body.to_string()),
    )
    .await;
    answer
        .cookies
        .iter()
        .find(|cookie| cookie.starts_with("organizr_token_"))
        .and_then(|cookie| cookie.split('=').next())
        .expect("the login sets the session cookie")
        .to_owned()
}

fn settings_with_id(id: &str) -> Settings {
    let mut settings = Settings::defaults();
    settings
        .set("uuid", &json!(id))
        .expect("the value is valid");
    settings
}

#[tokio::test]
async fn replaced_settings_are_used_by_the_next_request() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let state = AppState::new(database.clone(), Settings::defaults());
        let app = router(state.clone()).expect("the route table is valid");
        assert_eq!(session_cookie_of_a_login(&app).await, "organizr_token_temp");

        state.replace_settings(settings_with_id(FIRST_ID));
        assert_eq!(
            session_cookie_of_a_login(&app).await,
            format!("organizr_token_{FIRST_ID}")
        );

        state.replace_settings(settings_with_id(SECOND_ID));
        assert_eq!(
            session_cookie_of_a_login(&app).await,
            format!("organizr_token_{SECOND_ID}")
        );
    })
    .await;
}

#[tokio::test]
async fn settings_someone_already_holds_do_not_change() {
    on_each_engine(async |database| {
        let state = AppState::new(database.clone(), settings_with_id(FIRST_ID));

        // A request that is under way keeps what it got.
        let held = state.settings();
        state.replace_settings(settings_with_id(SECOND_ID));

        assert_eq!(held.install_id, FIRST_ID);
        assert_eq!(state.settings().install_id, SECOND_ID);
        // A copy of the state, as every request gets one, sees the same settings.
        assert_eq!(state.clone().settings().install_id, SECOND_ID);
    })
    .await;
}

#[tokio::test]
async fn reload_reads_what_the_database_holds_now() {
    on_each_engine(async |database| {
        seed::installed(database).await;
        let state = AppState::new(database.clone(), Settings::defaults());
        let app = router(state.clone()).expect("the route table is valid");
        database
            .settings()
            .put("uuid", &json!(FIRST_ID).to_string())
            .await
            .expect("the setting can be stored");
        database
            .settings()
            .put("rememberMeDays", "\"not a number\"")
            .await
            .expect("the setting can be stored");
        // Not reloaded yet: the application still runs with what it started with.
        assert_eq!(session_cookie_of_a_login(&app).await, "organizr_token_temp");

        let problems = state.reload_settings().await.expect("the reload works");

        assert_eq!(
            session_cookie_of_a_login(&app).await,
            format!("organizr_token_{FIRST_ID}")
        );
        // The entry that cannot be used is reported, and its setting has the default.
        assert!(
            matches!(
                problems.as_slice(),
                [SettingsError::InvalidValue {
                    key: "rememberMeDays",
                    ..
                }]
            ),
            "{problems:?}"
        );
        assert_eq!(state.settings().remember_me_days, 7);

        let (read_again, _) = stored_settings(database).await.expect("the read works");
        assert_eq!(read_again.install_id, FIRST_ID);
    })
    .await;
}
