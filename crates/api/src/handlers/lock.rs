//! The lock screen: a user locks the own account, and opens it again with the
//! password. An Admin or a Co-Admin locks and opens the account of another user.
//!
//! A locked account stays logged in. The interface shows the lock screen for it, and
//! forward auth refuses it.

use std::time::{Duration, Instant};

use axum::Extension;
use axum::extract::{Path, State};
use organizr_auth::login_limit::{Attempt, LimitPolicy};
use organizr_auth::password::{self, PasswordCheck};
use organizr_domain::{Change, GroupLevel, User, UserChanges};

use crate::api_data::ApiData;
use crate::caller::Caller;
use crate::client::Client;
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::handlers::fields::Refusal;
use crate::handlers::users::{is_admin, user_at};
use crate::state::AppState;

/// The level that locks and opens the accounts of other users.
const MANAGER: GroupLevel = GroupLevel::CO_ADMIN;

/// Stores whether the account is locked, and says what was done.
async fn set_locked(
    state: &AppState,
    user: &User,
    locked: bool,
) -> Result<Envelope<NoData>, Refusal> {
    let changes = UserChanges {
        locked: Change::Set(locked),
        ..UserChanges::none()
    };
    if state
        .database()
        .users()
        .update(user.id, changes)
        .await?
        .is_none()
    {
        return Err(Refusal::not_found("User not found"));
    }
    let (word, message) = if locked {
        ("locked", "User account locked")
    } else {
        ("unlocked", "User account unlocked")
    };
    tracing::info!(user = %user.username, "an account was {word}");
    Ok(Envelope::success(NoData).with_message(message))
}

/// The account of the caller. A script with the API key has none.
async fn own_account(state: &AppState, caller: &Caller) -> Result<User, Refusal> {
    caller.require(GroupLevel::LOGGED_IN)?;
    let Some(me) = caller.user() else {
        return Err(Refusal::conflict("Locking not allowed on Guest users"));
    };
    state
        .database()
        .users()
        .find(me.authenticated.user.id)
        .await?
        .ok_or_else(|| Refusal::not_found("User not found"))
}

/// `POST /users/lock`: locks the account of the caller.
pub async fn lock_own(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<NoData>, Refusal> {
    let user = own_account(&state, &caller).await?;
    set_locked(&state, &user, true).await
}

/// `POST /users/unlock`: opens the account of the caller, for the field `password`.
///
/// Wrong passwords are counted like failed logins of the account. When the count
/// reaches the `loginAttempts` setting, the answer is `403 "User is locked out"` for
/// `loginLockout` milliseconds. The PHP application has no such limit.
pub async fn unlock_own(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    data: ApiData,
) -> Result<Envelope<NoData>, Refusal> {
    let user = own_account(&state, &caller).await?;
    let Some(given) = data.text("password").filter(|text| !text.is_empty()) else {
        return Err(Refusal::unprocessable("Password Not Set"));
    };
    let limiter = state.login_limiter();
    let attempt = Attempt::new(client.address, &user.username);
    let settings = state.settings();
    let policy = LimitPolicy {
        failures: settings.login_attempts,
        lockout: Duration::from_millis(settings.login_lockout),
    };
    // The attempt counts before the check of the password (D-139).
    let pending = limiter
        .begin(&attempt, policy, Instant::now())
        .map_err(|retry_after| Refusal::from(ApiError::LockedOut { retry_after }))?;
    if !matches!(
        password::verify(&given, &user.password_hash),
        PasswordCheck::Correct { .. }
    ) {
        tracing::warn!(user = %user.username, client = %client.address, "a wrong password for the lock screen");
        return Err(match pending.failed() {
            Some(retry_after) => ApiError::LockedOut { retry_after }.into(),
            None => Refusal::not_authorized("Password Incorrect"),
        });
    }
    pending.succeeded();
    set_locked(&state, &user, false).await
}

/// The user of the path, for a caller who manages users. Only an Admin locks or opens
/// the account of an Admin.
async fn managed(state: &AppState, caller: &Caller, id: &str) -> Result<User, Refusal> {
    caller.require(MANAGER)?;
    let database = state.database();
    let user = user_at(database, id)
        .await?
        .ok_or_else(|| Refusal::not_found("User not found"))?;
    let groups = database.groups().list().await?;
    if is_admin(&groups, &user) && caller.level() != GroupLevel::ADMIN {
        return Err(Refusal::not_authorized(
            "Only admins can lock or unlock admins",
        ));
    }
    Ok(user)
}

/// `POST /users/lock/{id}`: locks the account of a user.
pub async fn lock_user(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    let user = managed(&state, &caller, &id).await?;
    set_locked(&state, &user, true).await
}

/// `POST /users/unlock/{id}`: opens the account of a user.
pub async fn unlock_user(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Envelope<NoData>, Refusal> {
    let user = managed(&state, &caller, &id).await?;
    set_locked(&state, &user, false).await
}
