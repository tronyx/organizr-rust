//! Sessions: one per login of a user on a device.

use std::net::IpAddr;

use crate::{SessionId, Timestamp, UserId};

/// The stored form of a session token: its SHA-256 hash as 64 hexadecimal characters.
///
/// The token itself is only in the cookie of the browser. The database holds this hash,
/// so a copy of the database does not let anyone take over a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionTokenHash(String);

impl SessionTokenHash {
    /// Wraps a hash in its text form.
    pub const fn new(hash: String) -> Self {
        Self(hash)
    }

    /// The hash in its text form.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A login of a user on a device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// The identifier of the session.
    pub id: SessionId,
    /// The hash of the token that the browser holds.
    pub token_hash: SessionTokenHash,
    /// The user who logged in.
    pub user: UserId,
    /// The `User-Agent` header of the login request. The API calls it `browser`.
    pub user_agent: Option<String>,
    /// The address the login came from.
    pub ip: IpAddr,
    /// When the user logged in.
    pub created_at: Timestamp,
    /// The moment from which the session is no longer valid.
    pub expires_at: Timestamp,
}

/// The data for a session that does not exist yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSession {
    /// The hash of the token that the browser holds.
    pub token_hash: SessionTokenHash,
    /// The user who logged in.
    pub user: UserId,
    /// The `User-Agent` header of the login request.
    pub user_agent: Option<String>,
    /// The address the login came from.
    pub ip: IpAddr,
    /// When the user logged in.
    pub created_at: Timestamp,
    /// The moment from which the session is no longer valid.
    pub expires_at: Timestamp,
}
