//! Who makes a request: a logged-in user, a guest, or a script with the API key.
//!
//! A browser proves a login with the session cookie. A script proves its right with the
//! API key of the install, in the header `token` or in the query field `apikey`.
//! [`identify_caller`] reads both on every request and gives the handlers a [`Caller`].
//!
//! A session cookie that names no valid session is dropped: the answer tells the browser
//! to delete it, on every route, as the PHP application does. The next request of that
//! browser is then the request of a guest.

use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{Request, State};
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use organizr_auth::SessionToken;
use organizr_auth::api_key::is_api_key;
use organizr_auth::session::{self, Authenticated, BrowserRule, SessionCheck};
use organizr_domain::{GroupLevel, Timestamp};
use organizr_settings::Settings;

use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::error::ApiError;
use crate::state::AppState;

/// The most session cookies of a request that are looked at (D-145).
pub const MOST_SESSION_COOKIES: usize = 4;

/// The header that carries the API key.
const API_KEY_HEADER: &str = "token";

/// The query field that carries the API key when the header is not sent.
const API_KEY_QUERY_FIELD: &str = "apikey";

/// Who makes a request.
#[derive(Debug, Clone)]
pub struct Caller {
    /// What the session cookie says about the caller.
    pub identity: Identity,
    /// Whether the request carries the API key of the install. The key gives the access
    /// of an Admin, whoever the caller is.
    pub has_api_key: bool,
}

/// What the session cookie of a request says about the caller.
#[derive(Debug, Clone)]
pub enum Identity {
    /// The request has no session cookie.
    Guest,
    /// The request has a session cookie that names no valid session. The answer drops
    /// the cookie.
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
        if self.has_api_key {
            return GroupLevel::ADMIN;
        }
        match &self.identity {
            Identity::Guest | Identity::Rejected => GroupLevel::GUEST,
            Identity::User(caller) => caller.authenticated.group.level,
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
        match &self.identity {
            Identity::User(caller) => Some(caller),
            Identity::Guest | Identity::Rejected => None,
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
pub fn browser_rule<'a>(settings: &Settings, headers: &'a HeaderMap) -> BrowserRule<'a> {
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

/// The text that a request gives as the API key: the header `token`, or else the query
/// field `apikey`. As in the PHP application, the query is not read when the header is
/// sent, also when the header holds something else.
fn presented_api_key(headers: &HeaderMap, query: Option<&str>) -> Option<String> {
    if let Some(value) = headers.get(API_KEY_HEADER) {
        // A value that is not text is no key.
        return Some(value.to_str().unwrap_or_default().to_owned());
    }
    form_urlencoded::parse(query?.as_bytes())
        .filter(|(name, _)| name == API_KEY_QUERY_FIELD)
        .map(|(_, value)| value.into_owned())
        .last()
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
    let cookie_name = session_cookie_name(&settings);
    let headers = request.headers();
    let now = SystemTime::now();

    let has_api_key = presented_api_key(headers, request.uri().query())
        .is_some_and(|presented| is_api_key(&presented, &settings.api_key));

    // An empty cookie counts as no cookie, as in the PHP application. Each cookie of
    // the name counts, up to a few, and the first that names a session wins: another
    // host of the cookie domain could have planted one that comes first (D-145).
    let sent: Vec<&str> = cookies::cookie_values(headers, &cookie_name)
        .filter(|value| !value.is_empty())
        .take(MOST_SESSION_COOKIES)
        .collect();
    let mut identity = if sent.is_empty() {
        Identity::Guest
    } else {
        Identity::Rejected
    };
    let rule = browser_rule(&settings, headers);
    for token in sent.into_iter().filter_map(SessionToken::parse) {
        match session::check(state.database(), &token, now_stamp(now), rule).await {
            Ok(SessionCheck::Valid(authenticated)) => {
                identity = Identity::User(Box::new(LoggedInCaller {
                    token,
                    authenticated: *authenticated,
                }));
                break;
            }
            Ok(SessionCheck::Unknown | SessionCheck::OtherBrowser) => {}
            Err(error) => return ApiError::Database(error).into_response(),
        }
    }

    let drop_cookie = matches!(identity, Identity::Rejected).then(|| {
        let secure = request
            .extensions()
            .get::<Client>()
            .is_some_and(|client| client.https);
        CookieScope::of(headers, secure)
    });
    request.extensions_mut().insert(Caller {
        identity,
        has_api_key,
    });

    let mut response = next.run(request).await;

    if let Some(scope) = drop_cookie {
        match scope.delete_session(&cookie_name, now) {
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
    use axum::http::{HeaderMap, HeaderValue};
    use organizr_domain::GroupLevel;
    use organizr_settings::Settings;
    use serde_json::json;

    use super::{Caller, Identity, presented_api_key, session_cookie_name};
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
        for identity in [Identity::Guest, Identity::Rejected] {
            let caller = Caller {
                identity,
                has_api_key: false,
            };

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

    #[test]
    fn api_key_gives_the_level_of_an_admin_but_no_user() {
        for identity in [Identity::Guest, Identity::Rejected] {
            let caller = Caller {
                identity,
                has_api_key: true,
            };

            assert_eq!(caller.level(), GroupLevel::ADMIN);
            assert!(caller.require(GroupLevel::ADMIN).is_ok());
            assert!(caller.user().is_none());
        }
    }

    fn headers(token: Option<&'static str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            headers.insert("token", HeaderValue::from_static(token));
        }
        headers
    }

    #[test]
    fn api_key_is_read_from_the_header_or_else_from_the_query() {
        assert_eq!(
            presented_api_key(&headers(Some("from-header")), None).as_deref(),
            Some("from-header")
        );
        assert_eq!(
            presented_api_key(&headers(None), Some("a=1&apikey=from%2Dquery&z=2")).as_deref(),
            Some("from-query")
        );
        // The last field counts, as in PHP.
        assert_eq!(
            presented_api_key(&headers(None), Some("apikey=first&apikey=last")).as_deref(),
            Some("last")
        );
        assert_eq!(presented_api_key(&headers(None), None), None);
        assert_eq!(presented_api_key(&headers(None), Some("a=1")), None);
        // The field has exactly this name.
        assert_eq!(presented_api_key(&headers(None), Some("APIKEY=x")), None);
        assert_eq!(presented_api_key(&headers(None), Some("token=x")), None);
    }

    #[test]
    fn header_hides_the_query_also_when_it_holds_something_else() {
        assert_eq!(
            presented_api_key(&headers(Some("something-else")), Some("apikey=the-key")).as_deref(),
            Some("something-else")
        );
        assert_eq!(
            presented_api_key(&headers(Some("")), Some("apikey=the-key")).as_deref(),
            Some("")
        );
    }
}
