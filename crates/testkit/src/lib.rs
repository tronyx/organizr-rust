//! Test helpers for Organizr: a database per engine, the seed of the recorded cases and
//! the replay of recorded PHP answers.
//!
//! This crate is used by tests only. Its functions report a failure by panicking, as
//! test code does.

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    reason = "test helpers report a failure by panicking"
)]

pub mod engines;
pub mod homepage_services;
pub mod http;
pub mod login_servers;
pub mod oidc_provider;
pub mod outbox;
pub mod replay;
pub mod seed;
pub mod services;
pub mod smtp;

use axum::Router;
use organizr_api::{AppState, router};
use organizr_auth::setup_code::SetupCode;
use organizr_db::Database;
use organizr_settings::Settings;

/// The application on the given database, with every setting at its default.
pub fn app(database: &Database) -> Router {
    app_with(database, Settings::defaults())
}

/// The application on the given database, with the settings of an install that has been
/// set up: the defaults, [`seed::INSTALL_ID`] and [`seed::API_KEY`].
pub fn installed_app(database: &Database) -> Router {
    installed_app_with(database, &[])
}

/// As [`installed_app`], with other values for some settings: key and stored value.
pub fn installed_app_with(database: &Database, changed: &[(&str, serde_json::Value)]) -> Router {
    app_with(database, installed_settings(changed))
}

/// The settings of [`installed_app_with`].
pub fn installed_settings(changed: &[(&str, serde_json::Value)]) -> Settings {
    let mut settings = Settings::defaults();
    settings
        .set("uuid", &serde_json::Value::from(seed::INSTALL_ID))
        .expect("the install id is a valid setting");
    settings
        .set("organizrAPI", &serde_json::Value::from(seed::API_KEY))
        .expect("the API key is a valid setting");
    for (key, value) in changed {
        settings
            .set(key, value)
            .unwrap_or_else(|error| panic!("the setting `{key}` cannot be set: {error}"));
    }
    settings
}

/// As [`installed_app_with`], with a stand-in for the mailer. The outbox gets every
/// e-mail that the application sends.
pub fn installed_app_with_outbox(
    database: &Database,
    changed: &[(&str, serde_json::Value)],
) -> (Router, outbox::Outbox) {
    let (mailer, outbox) = outbox::Outbox::new();
    let state = state_with(database, installed_settings(changed)).with_mailer(mailer);
    (router(state).expect("the route table is valid"), outbox)
}

/// A value for the `loginAttempts` setting that no test reaches. For tests that fail
/// many logins from one address for another reason than the login limit.
pub const NO_LOGIN_LIMIT: &str = "1000000";

/// The application on the given database, with the given settings. The set-up wizard
/// asks for [`seed::SETUP_CODE`].
pub fn app_with(database: &Database, settings: Settings) -> Router {
    router(state_with(database, settings)).expect("the route table is valid")
}

/// The state of [`app_with`], for a test that changes it while the application runs.
pub fn state_with(database: &Database, settings: Settings) -> AppState {
    let setup_code =
        SetupCode::parse(seed::SETUP_CODE).expect("the set-up code of the tests is valid");
    AppState::new(database.clone(), settings, seed::secrets()).with_setup_code(setup_code)
}

/// As [`installed_app`], with a folder for the pictures that people upload.
pub fn installed_app_in(database: &Database, picture_folder: &std::path::Path) -> Router {
    let mut settings = Settings::defaults();
    settings
        .set("uuid", &serde_json::Value::from(seed::INSTALL_ID))
        .expect("the install id is a valid setting");
    settings
        .set("organizrAPI", &serde_json::Value::from(seed::API_KEY))
        .expect("the API key is a valid setting");
    let state = state_with(database, settings)
        .with_picture_folder(picture_folder.to_owned())
        .with_picture_cache(picture_folder.join("cache"));
    router(state).expect("the route table is valid")
}
