//! The proof that a change of the own account comes from the owner: the password of the
//! account, or a code of its second factor.
//!
//! A session alone does not change the password, the e-mail address, the second factor
//! or a provider link of its account, so that a stolen session does not become a lasting
//! hold on the account (D-185 to D-187). The checks use the words of the unlink from a
//! provider (D-105), and a wrong proof counts like a failed login of the account.

use std::time::{Duration, Instant};

use axum::http::StatusCode;
use organizr_auth::login::is_second_factor_code;
use organizr_auth::login_limit::{Attempt, LimitPolicy, Pending};
use organizr_auth::password::{self, InTurnError, PasswordCheck};
use organizr_domain::User;

use crate::caller::now_stamp;
use crate::client::Client;
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::state::AppState;

/// The answer when the password is missing.
pub const NO_PASSWORD: &str = "Enter the password of the account";
/// The answer when the password is wrong.
pub const WRONG_PASSWORD: &str = "The password is not the password of the account";
/// The answer when neither the password nor a code came.
pub const NO_PROOF: &str = "Enter the password of the account or a code of its second factor";
/// The answer when the code is wrong.
pub const WRONG_CODE: &str = "The code is not a code of the second factor of the account";

/// What a request brings as proof. Empty texts count as missing.
#[derive(Debug, Default)]
pub struct Proof {
    /// The password of the account.
    pub password: Option<String>,
    /// A code of the second factor of the account.
    pub code: Option<String>,
}

impl Proof {
    /// The proof of a request: the password and the code, each when it is not empty. A
    /// code may come with spaces around it, as for a login; a password is taken as it is.
    pub fn new(password: Option<String>, code: Option<String>) -> Self {
        Self {
            password: password.filter(|text| !text.is_empty()),
            code: code
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty()),
        }
    }
}

/// Starts the count of an attempt on the account, which its logins share.
fn begin<'a>(state: &'a AppState, client: &Client, user: &User) -> Result<Pending<'a>, Refusal> {
    let settings = state.settings();
    let policy = LimitPolicy {
        failures: settings.login_attempts,
        lockout: Duration::from_millis(settings.login_lockout),
    };
    state
        .login_limiter()
        .begin(
            &Attempt::for_account(client.address, user.id),
            policy,
            Instant::now(),
        )
        .map_err(|retry_after| Refusal::from(ApiError::LockedOut { retry_after }))
}

/// The answer for a wrong proof: the lock, when this failure reached the limit.
fn failed(pending: Pending<'_>, message: &str) -> Refusal {
    match pending.failed() {
        Some(retry_after) => ApiError::LockedOut { retry_after }.into(),
        None => Refusal::unprocessable(message),
    }
}

/// Whether the password is the password of the account. A password that cannot be
/// checked, such as the random one of an account that a provider made, is wrong.
async fn is_password(user: &User, given: String) -> Result<bool, Refusal> {
    match password::verify_in_turn(given, user.password_hash.clone()).await {
        Ok(check) => Ok(matches!(check, PasswordCheck::Correct { .. })),
        Err(InTurnError::Busy) => Err(ApiError::Busy.into()),
        Err(InTurnError::Task(error)) => {
            tracing::error!(%error, "a password check for a change of the own account stopped");
            Err(Refusal::Request(
                StatusCode::INTERNAL_SERVER_ERROR,
                "the request could not be completed".to_owned(),
            ))
        }
    }
}

/// Checks the password of the account.
pub async fn password(
    state: &AppState,
    client: &Client,
    user: &User,
    given: Option<String>,
) -> Result<(), Refusal> {
    let Some(given) = Proof::new(given, None).password else {
        return Err(Refusal::unprocessable(NO_PASSWORD));
    };
    let pending = begin(state, client, user)?;
    if is_password(user, given).await? {
        pending.succeeded();
        Ok(())
    } else {
        tracing::warn!(user = %user.username, client = %client.address, "a wrong password for a change of the own account");
        Err(failed(pending, WRONG_PASSWORD))
    }
}

/// Checks the password of the account, or else a code of its second factor. The
/// password counts when both come.
pub async fn password_or_code(
    state: &AppState,
    client: &Client,
    user: &User,
    proof: Proof,
) -> Result<(), Refusal> {
    match proof {
        Proof {
            password: Some(given),
            ..
        } => password(state, client, user, Some(given)).await,
        Proof {
            code: Some(code), ..
        } => {
            let pending = begin(state, client, user)?;
            let now = now_stamp(std::time::SystemTime::now());
            let valid = is_second_factor_code(state.database(), user, &code, state.secrets(), now)
                .await
                .map_err(ApiError::from)?;
            if valid {
                pending.succeeded();
                Ok(())
            } else {
                tracing::warn!(user = %user.username, client = %client.address, "a wrong code for a change of the own account");
                Err(failed(pending, WRONG_CODE))
            }
        }
        Proof { .. } => Err(Refusal::unprocessable(NO_PROOF)),
    }
}
