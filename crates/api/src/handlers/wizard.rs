//! `POST /wizard`: setting up a new install.

use std::time::{Duration, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::install::{self, InstallError, SetUp};
use organizr_auth::login::{self, Device};
use organizr_db::Engine;

use crate::api_data::ApiData;
use crate::caller::{now_stamp, session_cookie_name};
use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::envelope::Envelope;
use crate::error::ApiError;
use crate::state::AppState;

/// How long the login lasts that the set-up ends with.
const ONE_DAY: Duration = Duration::from_hours(24);

/// The fields a set-up needs, in the order in which the PHP application asks for them.
const REQUIRED: [&str; 6] = [
    "license",
    "api",
    "registrationPassword",
    "username",
    "password",
    "email",
];

/// The name the answers of the API use for an engine.
pub const fn engine_name(engine: Engine) -> &'static str {
    match engine {
        Engine::Sqlite => "sqlite",
        Engine::Postgres => "postgresql",
    }
}

/// The engine that a `driver` of the PHP set-up wizard names, when this application has
/// that engine.
fn engine_of_driver(driver: &str) -> Option<Engine> {
    match driver.to_lowercase().as_str() {
        "sqlite" | "sqlite3" => Some(Engine::Sqlite),
        "postgre" | "postgres" | "postgresql" => Some(Engine::Postgres),
        _ => None,
    }
}

/// An answer of the wizard that is not a success. The PHP application puts `false` into
/// `data` for each of them.
fn refused(status: StatusCode, message: impl Into<String>) -> Response {
    Envelope::error(status, message, false).into_response()
}

/// `POST /wizard`: sets a new install up and logs the first Admin in.
///
/// Fields of the body: `setupCode`, `license`, `api`, `registrationPassword`, `username`,
/// `password` and `email`.
///
/// - `setupCode` is the code that the server wrote to its log when it started. It proves
///   that the caller is the person who runs the server. The PHP application has no such
///   code: there the first visitor of a new install becomes the Admin.
/// - The wizard does not choose the database. The engine is the one the server was
///   started with (see `ORGANIZR_DATABASE_URL`). A body that names another engine in
///   `driver` is refused, so that nobody sets an install up on a database they did not
///   mean to use. The other database fields of the PHP application, and `hashKey`, are
///   not read.
///
/// The answer is `true` and sets the session cookie. Once an install has a user, the
/// route answers `401`.
pub async fn wizard(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Response, ApiError> {
    let database = state.database();
    let already_set_up = || {
        refused(
            StatusCode::UNAUTHORIZED,
            "Endpoint disabled as database already exists",
        )
    };
    if database.install().is_done().await? {
        return Ok(already_set_up());
    }

    let sent_code = data.text("setupCode").unwrap_or_default();
    let code_is_right = state
        .setup_code()
        .is_some_and(|code| code.matches(&sent_code));
    if !code_is_right {
        tracing::warn!(client = %client.address, "a set-up was refused: the set-up code is missing or wrong");
        return Ok(refused(
            StatusCode::FORBIDDEN,
            "Setup code is missing or wrong. The server log shows the code",
        ));
    }

    let engine = database.engine();
    if let Some(driver) = data.text("driver").filter(|driver| !driver.is_empty())
        && engine_of_driver(&driver) != Some(engine)
    {
        return Ok(refused(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!(
                "[driver] is {driver}, but this server uses {}. Set ORGANIZR_DATABASE_URL \
                 before the first start to choose the database",
                engine_name(engine)
            ),
        ));
    }

    let field = |name: &str| data.text(name).unwrap_or_default();
    for name in REQUIRED {
        if field(name).is_empty() {
            return Ok(refused(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("[{name}] cannot be empty"),
            ));
        }
    }

    let now = SystemTime::now();
    let request = SetUp {
        username: field("username"),
        password: field("password"),
        email: field("email"),
        api_key: field("api"),
        license: field("license"),
        registration_password: field("registrationPassword"),
    };
    let admin = match install::set_up(database, request, now_stamp(now)).await {
        Ok(admin) => admin,
        // Another set-up finished while this one was under way.
        Err(InstallError::AlreadySetUp) => return Ok(already_set_up()),
        Err(error) => return Err(error.into()),
    };
    tracing::info!(
        admin = %admin.username,
        client = %client.address,
        database = engine_name(engine),
        "the install was set up"
    );

    // The set-up wrote settings, the id of the install among them. Requests from now on
    // must use them, and so must the name of the cookie that this answer sets.
    for problem in state.reload_settings().await? {
        tracing::warn!(%problem, "a stored setting is ignored");
    }

    let device = Device {
        ip: client.address,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    let logged_in = login::start_session(database, admin, device, now_stamp(now), ONE_DAY).await?;
    let session_cookie = CookieScope::of(&headers, client.https).set_hidden(
        &session_cookie_name(&state.settings()),
        logged_in.token.expose(),
        now + ONE_DAY,
    )?;
    let mut response = Envelope::success(true).into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use organizr_db::Engine;

    use super::engine_of_driver;

    #[test]
    fn driver_names_of_php_are_understood() {
        for driver in ["sqlite", "sqlite3", "SQLite3"] {
            assert_eq!(engine_of_driver(driver), Some(Engine::Sqlite), "{driver}");
        }
        for driver in ["postgre", "postgres", "postgresql", "PostgreSQL"] {
            assert_eq!(engine_of_driver(driver), Some(Engine::Postgres), "{driver}");
        }
        // PHP could also use MySQL. This application cannot.
        for driver in ["mysql", "mysqli", "oracle", "sqlite 3"] {
            assert_eq!(engine_of_driver(driver), None, "{driver}");
        }
    }
}
