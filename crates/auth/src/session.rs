//! Finding the user a session token belongs to.

use organizr_db::{Database, DbError};
use organizr_domain::{Group, Session, Timestamp, User};

use crate::session_token::SessionToken;

/// A user who is logged in, with the session that proves it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authenticated {
    /// The user.
    pub user: User,
    /// The group of the user, which decides what the user may do.
    pub group: Group,
    /// The session of the token.
    pub session: Session,
}

/// What a session token stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheck {
    /// The token belongs to a session that is valid for this request.
    Valid(Box<Authenticated>),
    /// No valid session has this token: it was never given out, it has expired, or the
    /// session was deleted.
    Unknown,
    /// The session exists, but it was made by another browser, and the install asks for
    /// the same browser (the `matchUserAgents` setting).
    OtherBrowser,
}

/// The browser rule of the `matchUserAgents` setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserRule<'a> {
    /// A session is valid from any browser.
    Any,
    /// A session is valid only when the request has the `User-Agent` header that the
    /// login had. Two requests without the header count as the same browser.
    Same(Option<&'a str>),
}

/// Looks up the session of a token as it is at `now`.
pub async fn check(
    database: &Database,
    token: &SessionToken,
    now: Timestamp,
    browser: BrowserRule<'_>,
) -> Result<SessionCheck, DbError> {
    let Some(session) = database.sessions().find_valid(&token.hash(), now).await? else {
        return Ok(SessionCheck::Unknown);
    };
    if let BrowserRule::Same(user_agent) = browser
        && session.user_agent.as_deref() != user_agent
    {
        tracing::warn!(
            session = %session.id,
            stored = ?session.user_agent,
            sent = ?user_agent,
            "a session was used from another browser than the one that logged in"
        );
        return Ok(SessionCheck::OtherBrowser);
    }

    // The database deletes the sessions of a deleted user, and refuses to delete the
    // group of a user. A missing record here means the session ended in between.
    let Some(user) = database.users().find(session.user).await? else {
        return Ok(SessionCheck::Unknown);
    };
    let Some(group) = database.groups().find(user.group).await? else {
        return Ok(SessionCheck::Unknown);
    };

    Ok(SessionCheck::Valid(Box::new(Authenticated {
        user,
        group,
        session,
    })))
}
