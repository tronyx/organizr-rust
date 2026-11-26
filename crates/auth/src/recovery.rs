//! A new password for a person who forgot the password of the account.
//!
//! The person names the e-mail address of the account and gets a new password by
//! e-mail. The PHP application replaces the password of the account at once. So anybody
//! who knows the address of a person can lock that person out, again and again.
//!
//! Here the password from the e-mail works next to the password of the account, for one
//! day. The first login with it makes it the password of the account. Until then the old
//! password works. And an account gets a new recovery password only once in five
//! minutes, so nobody can fill the mailbox of another person.

use std::time::Duration;

use organizr_db::{Database, DbError, Recovery};
use organizr_domain::{Timestamp, User};
use thiserror::Error;

use crate::password::{self, HashError, InTurnError, PasswordCheck};

/// How long a recovery password works.
pub const LIFETIME: Duration = Duration::from_hours(24);

/// How long an account waits for its next recovery password.
pub const PAUSE: Duration = Duration::from_mins(5);

/// The length of a recovery password.
const LENGTH: usize = 12;

/// The characters of a recovery password: letters and digits that a person does not
/// mix up when reading them, so no `0`, `O`, `1`, `l` or `I`.
const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";

/// Why a recovery could not start.
#[derive(Debug, Error)]
pub enum RecoveryError {
    /// The database failed.
    #[error(transparent)]
    Database(#[from] DbError),
    /// The new password could not be hashed.
    #[error(transparent)]
    Hash(#[from] HashError),
    /// The system gave no random bytes.
    #[error("cannot get random bytes for a new password: {0}")]
    Random(#[source] getrandom::Error),
    /// The check or the hash of the password did not run: the server was busy with
    /// other password checks, or the work stopped (D-140).
    #[error(transparent)]
    Task(#[from] InTurnError),
}

/// A recovery that was started: the account, and the password for the e-mail.
pub struct Started {
    /// The account.
    pub user: User,
    /// The new password, in plain text. It goes into the e-mail and nowhere else.
    pub password: String,
}

impl std::fmt::Debug for Started {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Started")
            .field("user", &self.user.username)
            .finish_non_exhaustive()
    }
}

fn seconds(duration: Duration) -> i64 {
    i64::try_from(duration.as_secs()).unwrap_or(i64::MAX)
}

/// A new random password.
pub(crate) fn new_password() -> Result<String, RecoveryError> {
    // Bytes above this would make the first characters of the alphabet more likely.
    let limit = u8::try_from(256 - 256 % ALPHABET.len()).unwrap_or(u8::MAX);
    let mut password = String::with_capacity(LENGTH);
    while password.len() < LENGTH {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes).map_err(RecoveryError::Random)?;
        for byte in bytes.into_iter().filter(|byte| *byte < limit) {
            if let Some(character) = ALPHABET.get(usize::from(byte) % ALPHABET.len())
                && password.len() < LENGTH
            {
                password.push(char::from(*character));
            }
        }
    }
    Ok(password)
}

/// Starts a recovery for the account with the given e-mail address, whatever its letter
/// case. Returns `None` when no account has the address, or when the account got a
/// recovery password less than [`PAUSE`] ago.
pub async fn start(
    database: &Database,
    email: &str,
    now: Timestamp,
) -> Result<Option<Started>, RecoveryError> {
    let Some(user) = database.users().find_by_email(email).await? else {
        return Ok(None);
    };
    if let Some(earlier) = database.users().recovery_of(user.id).await? {
        let made_at = earlier.expires_at.unix_seconds() - seconds(LIFETIME);
        if now.unix_seconds() < made_at + seconds(PAUSE) {
            tracing::info!(
                user = %user.username,
                "a recovery password was asked for again too soon; none was made"
            );
            return Ok(None);
        }
    }

    let password = new_password()?;
    let candidate = password.clone();
    let password_hash = password::hash_in_turn(candidate).await??;
    let recovery = Recovery {
        password_hash,
        expires_at: Timestamp::from_unix_seconds(now.unix_seconds() + seconds(LIFETIME)),
    };
    database.users().set_recovery(user.id, &recovery).await?;
    tracing::info!(user = %user.username, "a recovery password was made for the user");
    Ok(Some(Started { user, password }))
}

/// Whether `password` is the recovery password of the user and still works. When it
/// is, it becomes the password of the account.
pub(crate) async fn accept(
    database: &Database,
    user: &User,
    password: &str,
    now: Timestamp,
) -> Result<bool, DbError> {
    let Some(recovery) = database.users().recovery_of(user.id).await? else {
        return Ok(false);
    };
    if now.unix_seconds() >= recovery.expires_at.unix_seconds() {
        return Ok(false);
    }
    let candidate = password.to_owned();
    // A check that could not run counts as a wrong password.
    let check = password::verify_in_turn(candidate, recovery.password_hash)
        .await
        .unwrap_or(PasswordCheck::Wrong);
    if !matches!(check, PasswordCheck::Correct { .. }) {
        return Ok(false);
    }
    database.users().use_recovery(user.id).await?;
    // The person may have asked because somebody else has the account, so the sessions
    // from before end; the login makes a new one (D-146).
    let ended = database.sessions().delete_all_of_user(user.id).await?;
    tracing::info!(
        user = %user.username,
        ended,
        "the user logged in with a recovery password, which is the password of the account now"
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{ALPHABET, LENGTH, new_password};

    #[test]
    fn new_password_is_random_and_easy_to_read() {
        let first = new_password().expect("a password can be made");
        let second = new_password().expect("a password can be made");

        assert_ne!(first, second);
        assert_eq!(first.len(), LENGTH);
        for password in [first, second] {
            assert!(
                password.bytes().all(|byte| ALPHABET.contains(&byte)),
                "{password}"
            );
        }
        for confusing in b"0O1lI" {
            assert!(!ALPHABET.contains(confusing));
        }
    }

    #[test]
    fn every_character_of_the_alphabet_comes_up() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..200 {
            seen.extend(new_password().expect("a password can be made").bytes());
        }

        assert_eq!(seen.len(), ALPHABET.len());
    }
}
