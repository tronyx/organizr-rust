//! `POST /users/register`: creating an account with the registration password.

use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::login::{self, Device};
use organizr_auth::login_limit::{Attempt, LimitPolicy};
use organizr_auth::registration::{self, Registration, RegistrationError};
use organizr_domain::PasswordHash;
use serde::Serialize;

use crate::api_data::ApiData;
use crate::caller::{now_stamp, session_cookie_name};
use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::state::AppState;

const ONE_DAY: Duration = Duration::from_hours(24);

/// The longest a login lasts, in days, whatever the setting says.
const MOST_DAYS: u32 = 365;

/// What wrong registration passwords are counted under, next to the address of the
/// client. It holds a character that no name of a user can hold, because the space
/// around a name is cut off.
const LIMIT_KEY: &str = "\0registration password";

/// The `data` of the answer for a text that is not an e-mail address.
#[derive(Serialize)]
struct InvalidEmail {
    email: String,
}

/// An error answer without data.
fn refused(status: StatusCode, message: impl Into<String>) -> Response {
    Envelope::error(status, message, NoData).into_response()
}

/// `POST /users/register`: creates an account in the default group for a person who
/// knows the registration password, and logs that person in.
///
/// Fields of the body: `email`, `username`, `password` and `registrationPassword`. A
/// field that is missing, empty or the text `0` counts as not supplied, as in PHP.
///
/// Wrong registration passwords are counted like failed logins: by the address of the
/// client, and for the install as a whole. When a count reaches the `loginAttempts`
/// setting, the answer is `403 "User is locked out"` for `loginLockout` milliseconds.
/// The PHP application has no such limit.
pub async fn register(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Response, ApiError> {
    let settings = state.settings();
    // What PHP takes for "nothing": no value, the empty text, and the text `0`.
    let field = |name: &str| {
        data.text(name)
            .filter(|value| !value.is_empty() && value != "0")
    };
    let supplied = [
        ("email", "Email was not supplied"),
        ("username", "Username was not supplied"),
        ("password", "Password was not supplied"),
        (
            "registrationPassword",
            "Registration Password was not supplied",
        ),
    ];
    for (name, message) in supplied {
        if field(name).is_none() {
            return Ok(refused(StatusCode::UNPROCESSABLE_ENTITY, message));
        }
    }

    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, LIMIT_KEY);
    if let Some(retry_after) = limiter.locked_for(&attempt, Instant::now()) {
        return Err(ApiError::LockedOut { retry_after });
    }

    let now = SystemTime::now();
    let request = Registration {
        username: field("username").unwrap_or_default(),
        password: field("password").unwrap_or_default(),
        email: field("email").unwrap_or_default(),
        registration_password: field("registrationPassword").unwrap_or_default(),
    };
    let registration_hash = PasswordHash::new(settings.registration_password_hash.clone());
    let conflict = |message: String| refused(StatusCode::CONFLICT, message);
    let user = match registration::register(
        state.database(),
        request,
        &registration_hash,
        now_stamp(now),
    )
    .await
    {
        Ok(user) => user,
        Err(RegistrationError::WrongRegistrationPassword) => {
            tracing::warn!(client = %client.address, "a registration failed: the registration password is wrong");
            let policy = LimitPolicy {
                failures: settings.login_attempts,
                lockout: Duration::from_millis(settings.login_lockout),
            };
            return match limiter.record_failure(&attempt, policy, Instant::now()) {
                Some(retry_after) => Err(ApiError::LockedOut { retry_after }),
                None => Ok(refused(
                    StatusCode::UNAUTHORIZED,
                    "Registration Password was incorrect",
                )),
            };
        }
        Err(RegistrationError::EmptyUsername) => {
            return Ok(conflict("Username was set but empty".to_owned()));
        }
        Err(RegistrationError::InvalidEmail { email }) => {
            return Ok(Envelope::error(
                StatusCode::CONFLICT,
                "Email is not a valid email",
                InvalidEmail { email },
            )
            .into_response());
        }
        Err(RegistrationError::Taken { username, email }) => {
            return Ok(conflict(format!(
                "Username: {username} or Email: {email} is already taken"
            )));
        }
        Err(RegistrationError::TooLong { limit }) => {
            return Ok(conflict(format!("String is over limit of: {limit}")));
        }
        Err(error) => return Err(error.into()),
    };
    tracing::info!(user = %user.username, client = %client.address, "a user registered");

    // A new account stays logged in as a login with "remember me" does.
    let lifetime = ONE_DAY * settings.remember_me_days.min(MOST_DAYS);
    let device = Device {
        ip: client.address,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };
    let logged_in =
        login::start_session(state.database(), user, device, now_stamp(now), lifetime).await?;
    let session_cookie = CookieScope::of(&headers, client.https).set_hidden(
        &session_cookie_name(&settings),
        logged_in.token.expose(),
        now + lifetime,
    )?;
    let mut response = Envelope::success(NoData)
        .with_message("User created")
        .into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}
