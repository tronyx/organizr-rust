//! Who makes a request: a logged-in user or a guest.
//!
//! A browser proves a login with the session cookie. [`identify_caller`] reads the cookie
//! on every request, looks the session up and gives the handlers a [`Caller`].
//!
//! A session cookie that names no valid session is dropped: the answer tells the browser
//! to delete it, on every route, as the PHP application does. The next request of that
//! browser is then the request of a guest.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::http::header;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use organizr_auth::SessionToken;
use organizr_auth::session::{self, Authenticated, BrowserRule, SessionCheck};
use organizr_domain::{GroupLevel, Timestamp};
use organizr_settings::Settings;

use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::error::ApiError;
use crate::state::AppState;

/// Who makes a request.
#[derive(Debug, Clone)]
pub enum Caller {
    /// The request has no session cookie.
    Guest,
    /// The request has a session cookie that names no valid session. The caller has the
    /// rights of a guest, and the answer drops the cookie.
    Rejected,
    /// A user with a valid session.
    User(Box<LoggedInCaller>),
}

/// A caller with a valid session.
#[derive(Debug, Clone)]
pub struct LoggedInCaller {
    /// The token of the session cookie.
    pub token: SessionToken,
    /// The user, the group and the session.
    pub authenticated: Authenticated,
}

impl Caller {
    /// The access level of the caller. A lower number means more access.
    pub fn level(&self) -> GroupLevel {
        match self {
            Self::Guest | Self::Rejected => GroupLevel::GUEST,
            Self::User(caller) => caller.authenticated.group.level,
        }
    }

    /// Lets the caller pass when the level of the caller is `least_access` or one with
    /// more access. Otherwise the answer is `401 "Not Authorized"`.
    pub fn require(&self, least_access: GroupLevel) -> Result<(), ApiError> {
        if self.level() <= least_access {
            Ok(())
        } else {
            Err(ApiError::NotAuthorized)
        }
    }

    /// The logged-in caller, when there is one.
    pub fn user(&self) -> Option<&LoggedInCaller> {
        match self {
            Self::User(caller) => Some(caller),
            Self::Guest | Self::Rejected => None,
        }
    }
}

/// The name of the session cookie. It ends with the id of the install, so two installs
/// on one domain do not take each other's cookie.
pub fn session_cookie_name(settings: &Settings) -> String {
    if settings.install_id.is_empty() {
        "organizr_token_temp".to_owned()
    } else {
        format!("organizr_token_{}", settings.install_id)
    }
}

/// The current time as the database stores times.
pub fn now_stamp(now: SystemTime) -> Timestamp {
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    Timestamp::from_unix_seconds(i64::try_from(since_epoch.as_secs()).unwrap_or(i64::MAX))
}

/// The browser rule that the settings ask for, for a request with the given headers.
pub fn browser_rule<'a>(
    settings: &Settings,
    headers: &'a axum::http::HeaderMap,
) -> BrowserRule<'a> {
    if settings.match_user_agents {
        BrowserRule::Same(
            headers
                .get(header::USER_AGENT)
                .and_then(|value| value.to_str().ok()),
        )
    } else {
        BrowserRule::Any
    }
}

/// Works out who makes each request and makes it available to the handlers as a
/// [`Caller`] in the request extensions. Use it as a middleware, inside the one that
/// identifies the client.
pub async fn identify_caller(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let settings = state.settings();
    let cookie_name = session_cookie_name(settings);
    let headers = request.headers();
    let now = SystemTime::now();

    // An empty cookie counts as no cookie, as in the PHP application.
    let sent = cookies::cookie_value(headers, &cookie_name).filter(|value| !value.is_empty());
    let caller = match sent.map(SessionToken::parse) {
        None => Caller::Guest,
        Some(None) => Caller::Rejected,
        Some(Some(token)) => {
            let rule = browser_rule(settings, headers);
            match session::check(state.database(), &token, now_stamp(now), rule).await {
                Ok(SessionCheck::Valid(authenticated)) => Caller::User(Box::new(LoggedInCaller {
                    token,
                    authenticated: *authenticated,
                })),
                Ok(SessionCheck::Unknown | SessionCheck::OtherBrowser) => Caller::Rejected,
                Err(error) => return ApiError::Database(error).into_response(),
            }
        }
    };

    let drop_cookie = matches!(caller, Caller::Rejected).then(|| {
        let secure = request
            .extensions()
            .get::<Client>()
            .is_some_and(|client| client.https);
        CookieScope::of(headers, secure)
    });
    request.extensions_mut().insert(caller);

    let mut response = next.run(request).await;

    if let Some(scope) = drop_cookie {
        match scope.delete(&cookie_name, now) {
            // Before the cookies of the handler: a login in the same request sets the
            // cookie again, and the browser keeps the last one.
            Ok(values) => cookies::prepend(&mut response, values),
            Err(error) => tracing::error!(%error, "the session cookie could not be dropped"),
        }
    }
    response
}

#[cfg(test)]
mod tests {
    use organizr_domain::GroupLevel;
    use organizr_settings::Settings;
    use serde_json::json;

    use super::{Caller, session_cookie_name};
    use crate::error::ApiError;

    #[test]
    fn session_cookie_is_named_after_the_install() {
        let mut settings = Settings::defaults();
        assert_eq!(session_cookie_name(&settings), "organizr_token_temp");

        settings
            .set("uuid", &json!("0b1f6a3e-7c1d-4a52-9d55-3f2a8c9e1b47"))
            .expect("the value is valid");
        assert_eq!(
            session_cookie_name(&settings),
            "organizr_token_0b1f6a3e-7c1d-4a52-9d55-3f2a8c9e1b47"
        );
    }

    #[test]
    fn caller_without_a_valid_session_has_the_level_of_a_guest() {
        for caller in [Caller::Guest, Caller::Rejected] {
            assert_eq!(caller.level(), GroupLevel::GUEST);
            assert!(caller.user().is_none());
            assert!(caller.require(GroupLevel::GUEST).is_ok());
            let logged_in_only = GroupLevel::new(998).expect("the level is valid");
            assert!(matches!(
                caller.require(logged_in_only),
                Err(ApiError::NotAuthorized)
            ));
            assert!(matches!(
                caller.require(GroupLevel::ADMIN),
                Err(ApiError::NotAuthorized)
            ));
        }
    }
}
