//! Logging in with a password that Organizr itself stores.

use std::net::IpAddr;
use std::sync::OnceLock;
use std::time::Duration;

use organizr_db::{Database, DbError};
use organizr_domain::{NewSession, PasswordHash, Session, Timestamp, User};
use thiserror::Error;

use crate::password::{self, HashError, InTurnError, PasswordCheck};
use crate::recovery::{self, RecoveryError};
use crate::registration::RegistrationError;
use crate::secrets::{SecretBox, SecretError, two_factor_name};
use crate::session_token::{SessionToken, TokenError};
use crate::two_factor;

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
    /// Whether the code of a second factor was checked for this login.
    pub second_factor_checked: bool,
}

/// Why a login failed.
#[derive(Debug, Error)]
pub enum LoginError {
    /// No user has the name or the address, or the password is not the one of the user.
    /// The caller must not learn which of the two it was.
    #[error("the name or the password is wrong")]
    WrongCredentials,

    /// The account has a second factor, and the login came without a code.
    #[error("the login needs the code of the second factor")]
    SecondFactorNeeded,

    /// The code of the second factor is wrong.
    #[error("the code of the second factor is wrong")]
    WrongSecondFactor,

    /// The login came with a token of a kind that the application does not know.
    #[error("the kind of the login token is not known")]
    UnknownTokenKind,

    /// The login came with a Plex token, and the `plexoAuth` setting is off.
    #[error("the login with a Plex token is switched off")]
    TokenLoginIsOff,

    /// The account for a user of another service could not be made.
    #[error("the account could not be made: {0}")]
    Account(#[source] Box<RegistrationError>),

    /// The password or the address for a new account could not be made.
    #[error(transparent)]
    NewAccount(RecoveryError),

    /// A secret of the user could not be sealed.
    #[error(transparent)]
    Secret(SecretError),

    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),

    /// The new hash of the password could not be made.
    #[error(transparent)]
    Hash(#[from] HashError),

    /// The session token could not be made.
    #[error(transparent)]
    Token(#[from] TokenError),

    /// The check or the hash of the password did not run: the server was busy with
    /// other password checks, or the work stopped (D-140).
    #[error(transparent)]
    Task(#[from] InTurnError),
}

/// What a login brings for the second factor of an account.
#[derive(Clone, Copy)]
pub struct SecondFactor<'a> {
    /// The code that came with the login. Empty when none came.
    pub code: &'a str,
    /// Opens the stored secret of the second factor.
    pub secrets: &'a SecretBox,
    /// Whether this login needs no code, for example because the `ignoreTFALocal`
    /// setting is on and the login comes from the local network.
    pub skip: bool,
}

impl<'a> SecondFactor<'a> {
    /// A login that came without a code.
    pub const fn without_code(secrets: &'a SecretBox) -> Self {
        Self {
            code: "",
            secrets,
            skip: false,
        }
    }
}

/// Checks the second factor of a user whose password was correct. Returns whether a
/// code was checked.
pub(crate) async fn check_second_factor(
    database: &Database,
    user: &User,
    second_factor: SecondFactor<'_>,
    now: Timestamp,
) -> Result<bool, LoginError> {
    let stored = database
        .users()
        .login_of(user.id)
        .await?
        .and_then(|login| login.two_factor);
    let Some(stored) = stored.filter(|_| !second_factor.skip) else {
        return Ok(false);
    };
    if second_factor.code.is_empty() {
        return Err(LoginError::SecondFactorNeeded);
    }

    let now = u64::try_from(now.unix_seconds()).unwrap_or_default();
    let step = if stored.kind == two_factor::KIND {
        second_factor
            .secrets
            .open(&two_factor_name(user.id), &stored.sealed_secret)
            .ok()
            .and_then(|secret| two_factor::step_of(&secret, second_factor.code, now).ok())
            .unwrap_or_else(|| {
                tracing::error!(
                    user = %user.username,
                    "the stored secret of the second factor of the user cannot be used"
                );
                None
            })
    } else {
        None
    };
    // A code works for one login. The sealed secret is new for each secret, so the
    // steps of an old secret do not count for a new one.
    let factor = format!("{} {}", user.id, stored.sealed_secret);
    let valid = step.is_some_and(|step| two_factor::take_step(&factor, step, now));
    if valid {
        Ok(true)
    } else {
        Err(LoginError::WrongSecondFactor)
    }
}

/// Logs a user in with a name, or an e-mail address, and a password.
///
/// The letter case of the name and of the address does not matter. When the password is
/// correct, a session that lasts for `lifetime` from `now` is stored and its token is
/// returned. When the stored hash was made with an older method, it is replaced by a new
/// hash of the same password.
///
/// An account with a second factor also needs its code. The password is checked first,
/// so a login with a wrong password does not tell whether the account has a second
/// factor.
pub async fn with_password(
    database: &Database,
    name_or_email: &str,
    password: &str,
    second_factor: SecondFactor<'_>,
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
    let check = password::verify_in_turn(candidate, stored_hash).await?;

    let (user, outdated, with_account_password) = match (user, check) {
        (Some(user), PasswordCheck::Correct { outdated }) => (user, outdated, true),
        // A person who forgot the password logs in with the password from the e-mail.
        (Some(user), _) if recovery::accept(database, &user, password, now).await? => {
            (user, false, false)
        }
        (Some(user), PasswordCheck::Unusable) => {
            tracing::warn!(
                user = %user.username,
                "the stored password of the user is not a hash that can be checked"
            );
            return Err(LoginError::WrongCredentials);
        }
        _ => return Err(LoginError::WrongCredentials),
    };
    let second_factor_checked = check_second_factor(database, &user, second_factor, now).await?;

    // The person knew the password after all, so the password from an e-mail is not
    // needed any more, and nobody else can use it.
    if with_account_password && database.users().clear_recovery(user.id).await? {
        tracing::info!(
            user = %user.username,
            "the user logged in with the password of the account; the recovery password ended"
        );
    }

    if outdated {
        let candidate = password.to_owned();
        let new_hash = password::hash_in_turn(candidate).await??;
        database
            .users()
            .set_password_hash(user.id, &new_hash)
            .await?;
        tracing::info!(user = %user.username, "the password hash of the user was renewed");
    }

    let mut logged_in = start_session(database, user, device, now, lifetime).await?;
    logged_in.second_factor_checked = second_factor_checked;
    Ok(logged_in)
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
        second_factor_checked: false,
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
