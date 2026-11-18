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
pub mod replay;
pub mod seed;

use axum::Router;
use organizr_api::{AppState, router};
use organizr_db::Database;
use organizr_settings::Settings;

/// The application on the given database, with every setting at its default.
pub fn app(database: &Database) -> Router {
    app_with(database, Settings::defaults())
}

/// The application on the given database, with the settings of an install that has been
/// set up: the defaults and [`seed::INSTALL_ID`].
pub fn installed_app(database: &Database) -> Router {
    let mut settings = Settings::defaults();
    settings
        .set("uuid", &serde_json::Value::from(seed::INSTALL_ID))
        .expect("the install id is a valid setting");
    app_with(database, settings)
}

/// The application on the given database, with the given settings.
pub fn app_with(database: &Database, settings: Settings) -> Router {
    router(AppState::new(database.clone(), settings)).expect("the route table is valid")
}
