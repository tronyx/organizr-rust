//! Logging in with an account that another service vouches for, for example Plex.
//!
//! The service has checked who the person is: by a password that it knows, or by a token
//! that it gave out. This module does what follows. It finds the Organizr account of the
//! person, or makes one, and starts the session.

use std::time::Duration;

use organizr_db::Database;
use organizr_domain::{Timestamp, User};

use crate::login::{self, Device, LoggedIn, LoginError, SecondFactor};
use crate::password::{self, PasswordCheck};
use crate::recovery;
use crate::registration::{self, NewAccount};
use crate::secrets::plex_token_name;

/// The domain of the address that an account gets when the service names none. The PHP
/// application uses it for the same purpose.
pub(crate) const PLACEHOLDER_DOMAIN: &str = "placeholder.eml";

/// An account as the other service describes it.
#[derive(Clone, Copy)]
pub struct ExternalAccount<'a> {
    /// The user name at the service.
    pub username: &'a str,
    /// The e-mail address at the service. Empty when the service gives none.
    pub email: &'a str,
    /// The password that the person entered, when the login was one with a password.
    /// It becomes the password of the Organizr account too, so that the person can log
    /// in with it when the service is away.
    pub password: Option<&'a str>,
    /// The Plex token of the account, when the service is Plex.
    pub plex_token: Option<&'a str>,
    /// The name that the person typed. When it is given, the account is the one with
    /// this user name or this e-mail address, as PHP finds it after a login with a
    /// password; `username` and `email` are then only those of a new account. Without
    /// it, the account is the one with `username`, or else with `email`.
    pub found_by: Option<&'a str>,
    /// Whether the login may open an Admin or Co-Admin account. A service where anyone
    /// can make an account, as Plex and Emby Connect, may not, unless the account is the
    /// owner that the settings name. A directory that the Admin runs (LDAP, FTP, a local
    /// Emby server) may.
    pub may_open_privileged: bool,
}

/// What a login through another service did.
pub struct ExternalLogin {
    /// The session.
    pub logged_in: LoggedIn,
    /// Whether the Organizr account was made by this login.
    pub created: bool,
}

/// Logs in the Organizr account of a person whom another service has checked. The
/// account is the one that [`ExternalAccount::found_by`] names, or else the one with
/// the user name or the e-mail address of the service, in any letter case. When there
/// is none, an account is made in the group of new users.
///
/// An Admin or Co-Admin account is opened only when
/// [`ExternalAccount::may_open_privileged`] allows it, and the password of the service
/// never replaces the password of such an account.
///
/// The second factor of an Organizr account is asked for as in every login, unless
/// `second_factor.skip` says that this login needs none.
pub async fn log_in(
    database: &Database,
    account: ExternalAccount<'_>,
    second_factor: SecondFactor<'_>,
    device: Device,
    now: Timestamp,
    lifetime: Duration,
) -> Result<ExternalLogin, LoginError> {
    let users = database.users();
    let mut user = users
        .find_by_login(account.found_by.unwrap_or(account.username))
        .await?;
    if user.is_none() && account.found_by.is_none() && !account.email.is_empty() {
        user = users.find_by_email(account.email).await?;
    }
    let password = account.password.filter(|password| !password.is_empty());

    let (user, created, mirrored) = if let Some(user) = user {
        let privileged = crate::oidc::is_privileged(database, &user).await?;
        if privileged && !account.may_open_privileged {
            tracing::warn!(
                user = %user.username,
                "a login through another service may not open an Admin or Co-Admin account"
            );
            return Err(LoginError::WrongCredentials);
        }
        (user, false, password.filter(|_| !privileged))
    } else {
        (
            new_account(database, account, password, now).await?,
            true,
            None,
        )
    };

    // The account changes only after its second factor passed, so a login that the
    // service vouched for but whose code is wrong stores nothing.
    let second_factor_checked =
        login::check_second_factor(database, &user, second_factor, now).await?;
    if let Some(password) = mirrored {
        mirror_password(database, &user, password).await?;
    }

    if let Some(token) = account.plex_token.filter(|token| !token.is_empty()) {
        let name = plex_token_name(user.id);
        let stored = users
            .login_of(user.id)
            .await?
            .and_then(|login| login.sealed_plex_token)
            .and_then(|sealed| second_factor.secrets.open(&name, &sealed).ok());
        if stored.as_deref() != Some(token) {
            let sealed = second_factor
                .secrets
                .seal(&name, token)
                .map_err(LoginError::Secret)?;
            users.set_plex_token(user.id, Some(sealed)).await?;
        }
    }

    let mut logged_in = login::start_session(database, user, device, now, lifetime).await?;
    logged_in.second_factor_checked = second_factor_checked;
    Ok(ExternalLogin { logged_in, created })
}

/// Makes the password that the service took the password of the Organizr account.
async fn mirror_password(
    database: &Database,
    user: &User,
    password: &str,
) -> Result<(), LoginError> {
    let (candidate, stored) = (password.to_owned(), user.password_hash.clone());
    let check = password::verify_in_turn(candidate, stored).await?;
    if !matches!(check, PasswordCheck::Correct { .. }) {
        let candidate = password.to_owned();
        let hash = password::hash_in_turn(candidate).await??;
        database.users().set_password_hash(user.id, &hash).await?;
        // A new password ends the sessions from before, as a change in Organizr does;
        // the login makes its own session after this (D-146).
        let ended = database.sessions().delete_all_of_user(user.id).await?;
        tracing::info!(user = %user.username, ended, "the sessions of a user with a password from a login service were ended");
        tracing::info!(user = %user.username, "the password of the user was updated from the service");
    }
    Ok(())
}

/// Makes the Organizr account of a user of the service. Without a password from the
/// login, the account gets a random one, which nobody knows.
async fn new_account(
    database: &Database,
    account: ExternalAccount<'_>,
    password: Option<&str>,
    now: Timestamp,
) -> Result<User, LoginError> {
    let password = match password {
        Some(password) => password.to_owned(),
        None => recovery::new_password().map_err(LoginError::NewAccount)?,
    };
    let email = if account.email.is_empty() {
        let name = recovery::new_password().map_err(LoginError::NewAccount)?;
        format!("{name}@{PLACEHOLDER_DOMAIN}")
    } else {
        account.email.to_owned()
    };
    let new = NewAccount {
        username: account.username.to_owned(),
        password,
        email,
    };
    let user = registration::create_account(database, new, now)
        .await
        .map_err(|error| LoginError::Account(Box::new(error)))?;
    tracing::info!(user = %user.username, "an account was made for a user of the service");
    Ok(user)
}
