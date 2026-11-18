//! Routes that need no login and no data: the API root, `status`, `ping` and `rox`.

use axum::extract::State;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::envelope::{Envelope, JsonContentType};
use crate::error::ApiError;
use crate::handlers::wizard::engine_name;
use crate::state::AppState;

/// `GET /`: sends the caller to the status route.
///
/// The target is a fixed path, as in the PHP application.
pub async fn root() -> Response {
    redirect(StatusCode::FOUND, "/api/v2/status")
}

/// `GET /ping`: answers `pong`.
pub async fn ping() -> Envelope<&'static str> {
    Envelope::success("pong").with_content_type(JsonContentType::Plain)
}

/// `GET /rox`: a greeting the PHP application has always had.
pub async fn rox() -> Envelope<&'static str> {
    Envelope::success("https://www.roxinsocks.com/").with_message("rox in socks!")
}

/// `GET /roxinsocks`: a redirect the PHP application has always had.
pub async fn roxinsocks() -> Response {
    redirect(StatusCode::MOVED_PERMANENTLY, "https://roxinsocks.com")
}

/// `GET /status`: tells the front end whether the install needs the set-up wizard.
pub async fn status(State(state): State<AppState>) -> Result<Envelope<StatusData>, ApiError> {
    let is_set_up = state.database().install().is_done().await?;
    Ok(Envelope::success(StatusData {
        writable: true,
        min_version: true,
        os: if cfg!(windows) { "win" } else { "*nix" },
        php: "n/a",
        user_config_path_exists: is_set_up,
        action: if is_set_up {
            NextStep::Launch
        } else {
            NextStep::Wizard
        },
        database: engine_name(state.database().engine()),
    }))
}

/// The `data` of the status answer. The field names are those of the PHP application.
#[derive(Debug, Serialize)]
pub struct StatusData {
    /// Whether the application can write its data. A running application has opened its
    /// database for writing, so this is always true.
    writable: bool,
    /// Whether the PHP version is new enough. Kept for clients; always true.
    #[serde(rename = "minVersion")]
    min_version: bool,
    /// `*nix` or `win`, as the PHP application names the operating system.
    os: &'static str,
    /// The PHP version. Kept for clients; there is no PHP.
    php: &'static str,
    /// Whether the install has been set up.
    #[serde(rename = "userConfigPathExists")]
    user_config_path_exists: bool,
    /// What the front end does next.
    action: NextStep,
    /// The database engine the server uses: `sqlite` or `postgresql`. The PHP
    /// application has no such field. The set-up wizard shows it, because the engine is
    /// chosen before the first start and cannot be changed in the wizard.
    database: &'static str,
}

/// What the front end does after it has read the status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum NextStep {
    /// Show the set-up wizard.
    Wizard,
    /// Start the application.
    Launch,
}

/// A redirect as Slim sends it: no body and an HTML content type.
fn redirect(status: StatusCode, location: &'static str) -> Response {
    (
        status,
        [
            (header::LOCATION, location),
            (header::CONTENT_TYPE, "text/html; charset=UTF-8"),
        ],
    )
        .into_response()
}
