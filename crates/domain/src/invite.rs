//! Invites: codes that let a person get access to a media server.

/// An invite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    /// The id of the record.
    pub id: i64,
    /// The code that the invited person enters.
    pub code: String,
    /// When the invite was made, in seconds since 1970.
    pub created_at: i64,
    /// The e-mail address of the invited person.
    pub email: String,
    /// The name of the invited person.
    pub username: String,
    /// When the code was used, in seconds since 1970.
    pub used_at: Option<i64>,
    /// The account of the media server that used the code.
    pub used_by: Option<String>,
    /// The address that the code was used from.
    pub used_from: Option<String>,
    /// Whether the code can still be used.
    pub is_valid: bool,
    /// The kind of media server: `plex` or `emby`.
    pub kind: String,
    /// The user name of the person who made the invite.
    pub invited_by: String,
}

/// An invite that is not stored yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewInvite {
    /// The code.
    pub code: String,
    /// When the invite is made, in seconds since 1970.
    pub created_at: i64,
    /// The e-mail address of the invited person.
    pub email: String,
    /// The name of the invited person.
    pub username: String,
    /// The kind of media server.
    pub kind: String,
    /// The user name of the person who makes the invite.
    pub invited_by: String,
}
