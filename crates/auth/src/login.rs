//! Logging in with a password that Organizr itself stores.

use std::net::IpAddr;
use std::sync::OnceLock;
use std::time::Duration;

use organizr_db::{Database, DbError};
use organizr_domain::{NewSession, PasswordHash, Session, Timestamp, User};
use thiserror::Error;

use crate::password::{self, HashError, PasswordCheck};
use crate::session_token::{SessionToken, TokenError};

/// The device a login comes from. It is stored with the session, so users can tell
/// their sessions apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// The address of the client.
    pub ip: IpAddr,
    /// The `User-Agent` header of the login request.
    pub user_agent: Option<String>,
}

/// A login that succeeded.
#[derive(Debug)]
pub struct LoggedIn {
    /// The user who logged in.
    pub user: User,
    /// The token for the cookie of the browser. It is not stored anywhere else.
    pub token: SessionToken,
    /// The session as the database holds it.
    pub session: Session,
}

/// Why a login failed.
#[derive(Debug, Error)]
pub enum LoginError {
    /// No user has the name or the address, or the password is not the one of the user.
    /// The caller must not learn which of the two it was.
    #[error("the name or the password is wrong")]
    WrongCredentials,

    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),

    /// The new hash of the password could not be made.
    #[error(transparent)]
    Hash(#[from] HashError),

    /// The session token could not be made.
    #[error(transparent)]
    Token(#[from] TokenError),

    /// The task that checks or hashes the password did not finish.
    #[error("the password task did not finish: {0}")]
    Task(#[source] tokio::task::JoinError),
}

/// Logs a user in with a name, or an e-mail address, and a password.
///
/// The letter case of the name and of the address does not matter. When the password is
/// correct, a session that lasts for `lifetime` from `now` is stored and its token is
/// returned. When the stored hash was made with an older method, it is replaced by a new
/// hash of the same password.
pub async fn with_password(
    database: &Database,
    name_or_email: &str,
    password: &str,
    device: Device,
    now: Timestamp,
    lifetime: Duration,
) -> Result<LoggedIn, LoginError> {
    let user = database.users().find_by_login(name_or_email).await?;

    // An unknown name costs the time of a password check too, so the time an answer
    // takes does not tell whether a name exists.
    let stored_hash = user
        .as_ref()
        .map_or_else(placeholder_hash, |user| user.password_hash.clone());
    let candidate = password.to_owned();
    let check = tokio::task::spawn_blocking(move || password::verify(&candidate, &stored_hash))
        .await
        .map_err(LoginError::Task)?;

    let (user, outdated) = match (user, check) {
        (Some(user), PasswordCheck::Correct { outdated }) => (user, outdated),
        (Some(user), PasswordCheck::Unusable) => {
            tracing::warn!(
                user = %user.username,
                "the stored password of the user is not a hash that can be checked"
            );
            return Err(LoginError::WrongCredentials);
        }
        _ => return Err(LoginError::WrongCredentials),
    };

    if outdated {
        let candidate = password.to_owned();
        let new_hash = tokio::task::spawn_blocking(move || password::hash(&candidate))
            .await
            .map_err(LoginError::Task)??;
        database
            .users()
            .set_password_hash(user.id, &new_hash)
            .await?;
        tracing::info!(user = %user.username, "the password hash of the user was renewed");
    }

    start_session(database, user, device, now, lifetime).await
}

/// Logs a user in whose right to log in is already proven: stores a session that lasts
/// for `lifetime` from `now` and returns its token.
///
/// [`with_password`] calls this after it has checked the password. The set-up of an
/// install calls it for the Admin account it has just created.
pub async fn start_session(
    database: &Database,
    user: User,
    device: Device,
    now: Timestamp,
    lifetime: Duration,
) -> Result<LoggedIn, LoginError> {
    let token = SessionToken::generate()?;
    let lifetime = i64::try_from(lifetime.as_secs()).unwrap_or(i64::MAX);
    let session = database
        .sessions()
        .insert(NewSession {
            token_hash: token.hash(),
            user: user.id,
            user_agent: device.user_agent,
            ip: device.ip,
            created_at: now,
            expires_at: Timestamp::from_unix_seconds(now.unix_seconds().saturating_add(lifetime)),
        })
        .await?;

    Ok(LoggedIn {
        user,
        token,
        session,
    })
}

/// A hash that the password of an unknown name is checked against. No password is
/// accepted for it, because there is no user to log in.
///
/// It is made once, with the current settings, so that checking it takes as long as
/// checking a real hash. When it cannot be made, an empty value takes its place and the
/// check returns at once.
fn placeholder_hash() -> PasswordHash {
    static PLACEHOLDER: OnceLock<PasswordHash> = OnceLock::new();
    PLACEHOLDER
        .get_or_init(|| {
            password::hash("a password of no user")
                .unwrap_or_else(|_| PasswordHash::new(String::new()))
        })
        .clone()
}
