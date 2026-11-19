//! `POST /login`: logging in with a name and a password.

use std::time::{Duration, Instant, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::login::{self, Device, LoginError, SecondFactor};
use organizr_auth::login_limit::{Attempt, LimitPolicy};

use crate::api_data::ApiData;
use crate::caller::{now_stamp, session_cookie_name};
use crate::client::{Client, LocalNetworks};
use crate::cookies::{self, CookieScope};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::state::AppState;

/// How long a login lasts when the user did not ask to be remembered.
const ONE_DAY: Duration = Duration::from_hours(24);

/// The longest a login lasts, in days, whatever the setting says.
const MOST_DAYS: u32 = 365;

/// `POST /login`: checks a name, or an e-mail address, and a password. When they fit,
/// the answer sets the session cookie.
///
/// Fields of the body: `username`, `password` and, to stay logged in for the number of
/// days of the `rememberMeDays` setting instead of one day, `remember` with any value.
///
/// Failed logins are counted by the address of the client and by the name. When either
/// count reaches the `loginAttempts` setting, the answer is `403 "User is locked out"`,
/// for that login and for every login in the next `loginLockout` milliseconds, whatever
/// the password. The field `loginAttempts` of the body, in which the browser of the PHP
/// application sent its own count, is not read.
pub async fn login(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Response, ApiError> {
    let settings = state.settings();
    // A missing field counts as an empty text, as in the PHP application.
    let name = data.text("username").unwrap_or_default();
    let password = data.text("password").unwrap_or_default();
    let lifetime = if data.is_set("remember") {
        ONE_DAY * settings.remember_me_days.min(MOST_DAYS)
    } else {
        ONE_DAY
    };

    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, &name);
    if let Some(retry_after) = limiter.locked_for(&attempt, Instant::now()) {
        tracing::warn!(name = %name, client = %client.address, "a login was refused during a lock-out");
        return Err(ApiError::LockedOut { retry_after });
    }

    let now = SystemTime::now();
    let device = Device {
        ip: client.address,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };

    // The code of the second factor. With the `ignoreTFALocal` setting, a login from
    // the local network needs none.
    let code = data.text("tfaCode").unwrap_or_default();
    let second_factor = SecondFactor {
        code: &code,
        secrets: state.secrets(),
        skip: settings.ignore_tfa_local
            && LocalNetworks::from_setting(&settings.local_ip_list).contains(&client),
    };
    let result = login::with_password(
        state.database(),
        &name,
        &password,
        second_factor,
        device,
        now_stamp(now),
        lifetime,
    )
    .await;
    let logged_in = match result {
        Ok(logged_in) => logged_in,
        // A wrong code counts as a failed login, as a wrong password does.
        Err(error @ (LoginError::WrongCredentials | LoginError::WrongSecondFactor)) => {
            tracing::warn!(name = %name, client = %client.address, %error, "a login failed");
            let policy = LimitPolicy {
                failures: settings.login_attempts,
                lockout: Duration::from_millis(settings.login_lockout),
            };
            // The failure that reaches the limit already gets the lock-out answer.
            return Err(
                match limiter.record_failure(&attempt, policy, Instant::now()) {
                    Some(retry_after) => {
                        tracing::warn!(
                            name = %name,
                            client = %client.address,
                            seconds = retry_after.as_secs(),
                            "too many logins failed; the client and the name are locked out"
                        );
                        ApiError::LockedOut { retry_after }
                    }
                    None => error.into(),
                },
            );
        }
        Err(error) => return Err(error.into()),
    };
    limiter.record_success(&attempt);
    tracing::info!(user = %logged_in.user.username, client = %client.address, "a user logged in");

    let session_cookie = CookieScope::of(&headers, client.https).set_hidden(
        &session_cookie_name(&settings),
        logged_in.token.expose(),
        now + lifetime,
    )?;
    // The PHP application leaves the text of the code check in the answer.
    let answer = if logged_in.second_factor_checked {
        Envelope::success(NoData).with_message("Verification code verified")
    } else {
        Envelope::success(NoData)
    };
    let mut response = answer.into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}
