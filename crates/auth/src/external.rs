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
const PLACEHOLDER_DOMAIN: &str = "placeholder.eml";

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

    let (user, created) = if let Some(user) = user {
        if let Some(password) = password {
            mirror_password(database, &user, password).await?;
        }
        (user, false)
    } else {
        (new_account(database, account, password, now).await?, true)
    };

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

    let second_factor_checked =
        login::check_second_factor(database, &user, second_factor, now).await?;
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
    let check = tokio::task::spawn_blocking(move || password::verify(&candidate, &stored))
        .await
        .map_err(LoginError::Task)?;
    if !matches!(check, PasswordCheck::Correct { .. }) {
        let candidate = password.to_owned();
        let hash = tokio::task::spawn_blocking(move || password::hash(&candidate))
            .await
            .map_err(LoginError::Task)??;
        database.users().set_password_hash(user.id, &hash).await?;
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
