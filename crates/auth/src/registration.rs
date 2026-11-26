//! Registering an account with the registration password of the install.
//!
//! Anyone who knows the registration password can create an account in the default
//! group. The password is a setting of the install. Only its hash is stored.

use organizr_db::{Database, DbError};
use organizr_domain::{NewUser, PasswordHash, Timestamp, User, email};
use thiserror::Error;

use crate::install::gravatar;
use crate::password::{self, HashError, InTurnError, PasswordCheck};

/// The longest name and the longest e-mail address of an account, in bytes.
pub const LONGEST_NAME: usize = 50;
/// The longest password of an account, in bytes.
pub const LONGEST_PASSWORD: usize = 200;

/// What a person enters to register.
#[derive(Clone, PartialEq, Eq)]
pub struct Registration {
    /// The name of the new account.
    pub username: String,
    /// The password of the new account.
    pub password: String,
    /// The e-mail address of the new account.
    pub email: String,
    /// The registration password of the install, as the person entered it.
    pub registration_password: String,
}

/// The passwords are secrets, so `Debug` shows the name only.
impl std::fmt::Debug for Registration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Registration")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// Why a registration failed.
#[derive(Debug, Error)]
pub enum RegistrationError {
    /// The registration password is not the one of the install, or the install has none.
    #[error("the registration password is wrong")]
    WrongRegistrationPassword,

    /// The name is empty once the space around it is cut off.
    #[error("the name is empty")]
    EmptyUsername,

    /// The text is not an e-mail address.
    #[error("`{email}` is not an e-mail address")]
    InvalidEmail {
        /// The text as it was entered.
        email: String,
    },

    /// Another account has the name or the address, whatever the letter case.
    #[error("the name `{username}` or the address `{email}` is taken")]
    Taken {
        /// The name, without the space around it.
        username: String,
        /// The address as it would be stored.
        email: String,
    },

    /// The name, the address or the password is longer than `limit` bytes.
    #[error("a value is longer than {limit} bytes")]
    TooLong {
        /// The limit that was passed.
        limit: usize,
    },

    /// No group is the default group, so there is no group for a new account.
    #[error("no group is the default group")]
    NoDefaultGroup,

    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),

    /// The password of the new account could not be hashed.
    #[error(transparent)]
    Hash(#[from] HashError),

    /// The check or the hash of the password did not run: the server was busy with
    /// other password checks, or the work stopped (D-140).
    #[error(transparent)]
    Task(#[from] InTurnError),
}

/// Creates an account for a person who knows the registration password.
///
/// `registration_hash` is the stored hash of the registration password. An empty value
/// means that the install has no registration password, so nobody can register.
///
/// The registration password is checked first. Then [`create_account`] makes the account.
pub async fn register(
    database: &Database,
    request: Registration,
    registration_hash: &PasswordHash,
    now: Timestamp,
) -> Result<User, RegistrationError> {
    let Registration {
        username,
        password,
        email,
        registration_password,
    } = request;

    let stored = registration_hash.clone();
    let check = password::verify_in_turn(registration_password, stored).await?;
    if !matches!(check, PasswordCheck::Correct { .. }) {
        return Err(RegistrationError::WrongRegistrationPassword);
    }

    create_account(
        database,
        NewAccount {
            username,
            password,
            email,
        },
        now,
    )
    .await
}

/// The name, the password and the e-mail address for a new account.
#[derive(Clone, PartialEq, Eq)]
pub struct NewAccount {
    /// The name of the new account.
    pub username: String,
    /// The password of the new account.
    pub password: String,
    /// The e-mail address of the new account.
    pub email: String,
}

/// The password is a secret, so `Debug` shows the name only.
impl std::fmt::Debug for NewAccount {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NewAccount")
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

/// Creates an account in the default group, with the Gravatar picture of its address.
///
/// The checks run in the order of the PHP application: the name, the address, whether
/// name or address are taken, then the lengths.
pub async fn create_account(
    database: &Database,
    account: NewAccount,
    now: Timestamp,
) -> Result<User, RegistrationError> {
    let NewAccount {
        username,
        password,
        email,
    } = account;
    // The PHP application cuts the space off after it has looked for an empty name, and
    // so creates an account without a name for a name of spaces. Here that is refused.
    let username = email::php_trim(&username).to_owned();
    if username.is_empty() {
        return Err(RegistrationError::EmptyUsername);
    }
    if !email::is_valid(&email) {
        return Err(RegistrationError::InvalidEmail { email });
    }
    let email = email::sanitize(&email);

    let taken = || RegistrationError::Taken {
        username: username.clone(),
        email: email.clone(),
    };
    if database.users().is_taken(&username, &email).await? {
        return Err(taken());
    }
    for (value, limit) in [
        (&username, LONGEST_NAME),
        (&email, LONGEST_NAME),
        (&password, LONGEST_PASSWORD),
    ] {
        if value.len() > limit {
            return Err(RegistrationError::TooLong { limit });
        }
    }

    let group = database
        .groups()
        .find_default()
        .await?
        .ok_or(RegistrationError::NoDefaultGroup)?;
    let password_hash = hash_password(password).await?;

    let stored = database
        .users()
        .insert(NewUser {
            image: Some(gravatar(&email)),
            username: username.clone(),
            email: Some(email.clone()),
            password_hash,
            group: group.id,
            created_at: now,
        })
        .await;
    match stored {
        Ok(user) => Ok(user),
        // Another registration took the name in between.
        Err(DbError::Duplicate) => Err(taken()),
        Err(error) => Err(error.into()),
    }
}

/// Hashes a password for storing. The work runs away from the tasks that serve requests,
/// because it takes long on purpose.
pub async fn hash_password(password: String) -> Result<PasswordHash, RegistrationError> {
    let hash = password::hash_in_turn(password).await??;
    Ok(hash)
}
