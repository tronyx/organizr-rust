//! Users as the API shows them.

use organizr_domain::{Group, User};
use serde::Serialize;

use crate::handlers::token::date_and_time;

/// A user as the API shows it. The field names and their order are those of the PHP
/// application.
#[derive(Debug, Serialize)]
pub struct UserRow {
    id: i64,
    username: String,
    /// Always `null`: the hash of a password never leaves the server (D-05).
    password: Option<String>,
    email: Option<String>,
    /// Always `null`: the token of a service never leaves the server (D-05).
    plex_token: Option<String>,
    /// The name of the group of the user.
    group: Option<String>,
    /// The level of the group of the user.
    group_id: Option<u16>,
    locked: u8,
    image: Option<String>,
    /// When the user was stored, as `2026-11-18 19:02:06` in UTC.
    register_date: String,
    /// Which service checks the password of the user. Only `internal` exists so far.
    auth_service: &'static str,
}

impl UserRow {
    /// The row for a user. `groups` are the groups of the install, for the name and the
    /// level of the group of the user.
    pub fn of(user: User, groups: &[Group]) -> Self {
        let group = groups.iter().find(|group| group.id == user.group);
        Self {
            id: user.id.get(),
            username: user.username,
            password: None,
            email: user.email,
            plex_token: None,
            group: group.map(|group| group.name.clone()),
            group_id: group.map(|group| group.level.get()),
            locked: u8::from(user.locked),
            image: user.image,
            register_date: date_and_time(user.created_at),
            auth_service: "internal",
        }
    }
}
