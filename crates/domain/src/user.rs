//! Users.

use std::fmt;

use crate::{Change, GroupId, Timestamp, UserId};

/// A password in hashed form, as stored. Never the password itself.
///
/// Its `Debug` output hides the hash, so it cannot reach a log by accident.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    /// Wraps a hash string, for example one that starts with `$2y$` or `$argon2id$`.
    pub const fn new(hash: String) -> Self {
        Self(hash)
    }

    /// The hash string, for storing it or for checking a password against it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PasswordHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PasswordHash(<hidden>)")
    }
}

/// A user account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    /// The identifier of the user.
    pub id: UserId,
    /// The name to log in with. No two users share a name, whatever the letter case.
    pub username: String,
    /// The e-mail address.
    pub email: Option<String>,
    /// The hashed password.
    pub password_hash: PasswordHash,
    /// The group the user belongs to.
    pub group: GroupId,
    /// Whether the account is behind the lock screen.
    pub locked: bool,
    /// The picture: a path or a URL.
    pub image: Option<String>,
    /// When the account was created.
    pub created_at: Timestamp,
}

/// The data for a user that does not exist yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUser {
    /// The name to log in with.
    pub username: String,
    /// The e-mail address.
    pub email: Option<String>,
    /// The hashed password.
    pub password_hash: PasswordHash,
    /// The group the user belongs to.
    pub group: GroupId,
    /// The picture: a path or a URL.
    pub image: Option<String>,
    /// When the account was created.
    pub created_at: Timestamp,
}

/// Changes to a user. A field that is [`Change::Keep`] stays as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserChanges {
    /// The name to log in with.
    pub username: Change<String>,
    /// The e-mail address.
    pub email: Change<String>,
    /// The hashed password.
    pub password_hash: Change<PasswordHash>,
    /// The group the user belongs to.
    pub group: Change<GroupId>,
    /// Whether the account is locked.
    pub locked: Change<bool>,
    /// The picture: a path or a URL.
    pub image: Change<Option<String>>,
}

impl UserChanges {
    /// Changes that keep every field.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            username: Change::Keep,
            email: Change::Keep,
            password_hash: Change::Keep,
            group: Change::Keep,
            locked: Change::Keep,
            image: Change::Keep,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordHash;

    #[test]
    fn debug_output_hides_the_hash() {
        let hash = PasswordHash::new("$2y$10$abcdefghijklmnopqrstuv".to_owned());

        assert_eq!(format!("{hash:?}"), "PasswordHash(<hidden>)");
        assert_eq!(hash.expose(), "$2y$10$abcdefghijklmnopqrstuv");
    }
}
