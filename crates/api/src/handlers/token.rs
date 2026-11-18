//! The `/token` routes: who the caller is, whether a token is valid, and deleting sessions.
//!
//! The PHP application calls a session a "token", because its session cookie held a
//! signed token. The routes and the field names keep that word.

use std::net::IpAddr;
use std::time::SystemTime;

use axum::Extension;
use axum::body::Bytes;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use md5::{Digest, Md5};
use organizr_auth::SessionToken;
use organizr_auth::session::{self, Authenticated, SessionCheck};
use organizr_domain::{GroupLevel, Session, SessionId, Timestamp};
use serde::Serialize;

use crate::caller::{Caller, Identity, browser_rule, now_stamp};
use crate::client::Client;
use crate::envelope::{Envelope, NoContent};
use crate::error::ApiError;
use crate::state::AppState;

/// What the PHP application answers for a guest when the Guest group is missing.
const GUEST_GROUP_NAME: &str = "Guest";
const GUEST_GROUP_IMAGE: &str = "plugins/images/groups/guest.png";

/// The level a caller needs to delete a session: any logged-in user.
const LOGGED_IN: u16 = 998;

/// A user as the API describes the caller: the fields of the PHP application, in its
/// order and with its types.
#[derive(Debug, Serialize)]
pub struct UserObject<S> {
    /// The token that the caller sent to `POST /token/validate`. `null` in every other
    /// answer.
    token: Option<String>,
    #[serde(rename = "tokenDate")]
    token_date: Option<PhpDateTime>,
    #[serde(rename = "tokenExpire")]
    token_expire: Option<PhpDateTime>,
    username: String,
    /// The first five characters of the MD5 hash of the address of the caller.
    uid: String,
    group: String,
    #[serde(rename = "groupID")]
    group_id: u16,
    email: Option<String>,
    image: Option<String>,
    #[serde(rename = "userID")]
    user_id: Option<i64>,
    loggedin: bool,
    locked: Locked,
    #[serde(rename = "tokenList")]
    token_list: Option<Vec<S>>,
    #[serde(rename = "authService")]
    auth_service: Option<&'static str>,
}

/// A time as PHP writes a date object to JSON.
#[derive(Debug, Serialize)]
struct PhpDateTime {
    /// `2026-11-18 19:02:06.000000`, in UTC.
    date: String,
    timezone_type: u8,
    timezone: &'static str,
}

impl PhpDateTime {
    fn of(time: Timestamp) -> Self {
        Self {
            date: format!("{}.000000", date_and_time(time)),
            timezone_type: 1,
            timezone: "+00:00",
        }
    }
}

/// `locked` is `false` for a guest and the number 0 or 1 for a user: PHP passes on what
/// the database gives it.
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Locked {
    Guest(bool),
    User(u8),
}

/// A session in the list of `GET /token/me`.
#[derive(Debug, Serialize)]
pub struct SessionSummary {
    /// Always `null`. The PHP application put the token of the session here.
    token: Option<String>,
    ip: String,
    id: i64,
    expires: String,
    created: String,
    /// Whether this is the session of the request. The PHP application has no such
    /// field: its front end compared the tokens.
    current: bool,
}

/// A session in the list of `POST /token/validate`.
#[derive(Debug, Serialize)]
pub struct SessionRow {
    id: i64,
    /// The token that the caller sent, for the session of that token. `null` for every
    /// other session: only the browser of a session knows its token.
    token: Option<String>,
    user_id: i64,
    browser: Option<String>,
    ip: String,
    created: String,
    expires: String,
}

/// How a session becomes an entry of a session list. `token` is the token of the
/// session, when the caller may see it. `current` says whether it is the session that the
/// list is made for.
trait SessionEntry {
    fn of(session: &Session, token: Option<String>, current: bool) -> Self;
}

impl SessionEntry for SessionSummary {
    fn of(session: &Session, token: Option<String>, current: bool) -> Self {
        Self {
            token,
            ip: session.ip.to_string(),
            id: session.id.get(),
            expires: date_and_time(session.expires_at),
            created: date_and_time(session.created_at),
            current,
        }
    }
}

impl SessionEntry for SessionRow {
    fn of(session: &Session, token: Option<String>, _current: bool) -> Self {
        Self {
            id: session.id.get(),
            token,
            user_id: session.user.get(),
            browser: session.user_agent.clone(),
            ip: session.ip.to_string(),
            created: date_and_time(session.created_at),
            expires: date_and_time(session.expires_at),
        }
    }
}

/// A time as the PHP application stores it: `2026-11-18 19:02:06`, in UTC.
fn date_and_time(time: Timestamp) -> String {
    jiff::Timestamp::from_second(time.unix_seconds()).map_or_else(
        |_| String::new(),
        |time| time.strftime("%Y-%m-%d %H:%M:%S").to_string(),
    )
}

/// The `uid` of a caller: the start of the MD5 hash of the address, as text.
fn uid(address: IpAddr) -> String {
    let mut uid = hex::encode(Md5::digest(address.to_string().as_bytes()));
    uid.truncate(5);
    uid
}

/// Describes a logged-in user with the valid sessions of that user.
///
/// `shown_token` is the token of the session, when the answer may hold it: that is when
/// the caller sent the token in the request itself. A caller who is known by the session
/// cookie gets no token, so a script of the page cannot read it from the answer.
async fn describe_user<S: SessionEntry>(
    state: &AppState,
    authenticated: &Authenticated,
    shown_token: Option<&SessionToken>,
    client: Client,
    now: Timestamp,
) -> Result<UserObject<S>, ApiError> {
    let Authenticated {
        user,
        group,
        session,
    } = authenticated;
    let sessions = state
        .database()
        .sessions()
        .list_valid_of_user(user.id, now)
        .await?;
    let token_list = sessions
        .iter()
        .map(|listed| {
            let current = listed.id == session.id;
            let token = shown_token
                .filter(|_| current)
                .map(|token| token.expose().to_owned());
            S::of(listed, token, current)
        })
        .collect();

    Ok(UserObject {
        token: shown_token.map(|token| token.expose().to_owned()),
        token_date: Some(PhpDateTime::of(session.created_at)),
        token_expire: Some(PhpDateTime::of(session.expires_at)),
        username: user.username.clone(),
        uid: uid(client.address),
        group: group.name.clone(),
        group_id: group.level.get(),
        email: user.email.clone(),
        image: user.image.clone(),
        user_id: Some(user.id.get()),
        loggedin: true,
        locked: Locked::User(u8::from(user.locked)),
        token_list: Some(token_list),
        // Two-factor login, which changes this value, is not implemented yet.
        auth_service: Some("internal"),
    })
}

/// Describes a caller who is not logged in: a guest, or a script with the API key.
///
/// The PHP application calls a script with the API key the user "Organizr API" of the
/// group Admin. It has the picture of a guest, and it is not a user that is logged in.
async fn describe_guest(
    state: &AppState,
    client: Client,
    has_api_key: bool,
) -> Result<UserObject<SessionSummary>, ApiError> {
    let group = state
        .database()
        .groups()
        .find_by_level(GroupLevel::GUEST)
        .await?;
    let (name, image) = group.map_or_else(
        || {
            (
                GUEST_GROUP_NAME.to_owned(),
                Some(GUEST_GROUP_IMAGE.to_owned()),
            )
        },
        |group| (group.name, group.image),
    );

    let (username, group, level) = if has_api_key {
        ("Organizr API", "Admin".to_owned(), GroupLevel::ADMIN)
    } else {
        ("Guest", name, GroupLevel::GUEST)
    };

    Ok(UserObject {
        token: None,
        token_date: None,
        token_expire: None,
        username: username.to_owned(),
        uid: uid(client.address),
        group,
        group_id: level.get(),
        email: None,
        image,
        user_id: None,
        loggedin: false,
        locked: Locked::Guest(false),
        token_list: None,
        auth_service: None,
    })
}

/// `GET /token/me`: describes the caller.
///
/// A logged-in user gets the account and the list of their valid sessions. A guest gets
/// the description of a guest, and a script with the API key that of the user "Organizr
/// API". A caller whose session cookie was just dropped gets `null`, as in the PHP
/// application; the next request is that of a guest.
pub async fn me(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    Extension(caller): Extension<Caller>,
) -> Result<Envelope<Option<UserObject<SessionSummary>>>, ApiError> {
    let data = match &caller.identity {
        Identity::Guest => Some(describe_guest(&state, client, caller.has_api_key).await?),
        Identity::Rejected => None,
        Identity::User(caller) => {
            let now = now_stamp(SystemTime::now());
            Some(describe_user(&state, &caller.authenticated, None, client, now).await?)
        }
    };
    Ok(Envelope::success(data))
}

/// The `data` of `POST /token/validate`: the user of the token, or `false`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Validation {
    /// The token is not valid.
    Refused(bool),
    /// The user the token belongs to.
    Valid(Box<UserObject<SessionRow>>),
}

/// The field `Token` of a request, as PHP's `$_REQUEST` gives it: from a form body, or
/// else from the query. A JSON body is not read.
fn token_field(query: Option<&str>, headers: &HeaderMap, body: &[u8]) -> Option<String> {
    let field = |encoded: &[u8]| {
        form_urlencoded::parse(encoded)
            .filter(|(name, _)| name == "Token")
            .map(|(_, value)| value.into_owned())
            .last()
    };
    let is_form = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("x-www-form-urlencoded"));

    is_form
        .then(|| field(body))
        .flatten()
        .or_else(|| query.and_then(|query| field(query.as_bytes())))
}

/// `POST /token/validate`: tells whether a session token is valid, and whose it is.
///
/// The token is the field `Token` of the query or of a form body. Any caller may ask.
pub async fn validate(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Envelope<Validation>, ApiError> {
    let refused =
        |message: &str| Envelope::error(StatusCode::FORBIDDEN, message, Validation::Refused(false));

    // The PHP application crashes when the field is missing. Here a missing field is a
    // text that is no token.
    let text = token_field(query.as_deref(), &headers, &body).unwrap_or_default();
    let Some(token) = SessionToken::parse(&text) else {
        return Ok(refused("Token was invalid"));
    };

    let now = now_stamp(SystemTime::now());
    let rule = browser_rule(state.settings(), &headers);
    match session::check(state.database(), &token, now, rule).await? {
        SessionCheck::Valid(authenticated) => {
            let user = describe_user(&state, &authenticated, Some(&token), client, now).await?;
            Ok(Envelope::success(Validation::Valid(Box::new(user))).with_message("Token is valid"))
        }
        SessionCheck::Unknown => Ok(refused("Token was not in approved list")),
        // The PHP application sets no error for this case: the answer says "success"
        // and carries `false`.
        SessionCheck::OtherBrowser => Ok(Envelope::success(Validation::Refused(false))),
    }
}

/// `DELETE /token/{id}`: deletes one session of the caller.
///
/// The answer is `204` also when the caller has no session with that id, so it does not
/// tell which ids exist.
pub async fn revoke(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    revoke_by_text(&state, &caller, &id).await
}

/// [`revoke`] for an id that is given as text. Text that is no number names no session.
pub async fn revoke_by_text(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> Result<Response, ApiError> {
    caller.require(GroupLevel::new(LOGGED_IN).unwrap_or(GroupLevel::ADMIN))?;
    // What PHP counts as "no id": nothing, or the text `0`.
    if id.is_empty() || id == "0" {
        return Err(ApiError::Unprocessable("Id was not supplied"));
    }

    if let (Some(caller), Ok(id)) = (caller.user(), id.parse::<i64>()) {
        let user = caller.authenticated.user.id;
        let existed = state
            .database()
            .sessions()
            .delete_one_of_user(user, SessionId::new(id))
            .await?;
        if existed {
            tracing::info!(user = %caller.authenticated.user.username, session = id, "a session was deleted");
        }
    }
    Ok(NoContent.into_response())
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use axum::http::{HeaderMap, HeaderValue, header};
    use organizr_domain::Timestamp;

    use super::{PhpDateTime, date_and_time, token_field, uid};

    #[test]
    fn times_are_written_as_php_writes_them() {
        // 2026-11-18 19:02:06 UTC.
        let time = Timestamp::from_unix_seconds(1_795_028_526);

        assert_eq!(date_and_time(time), "2026-11-18 19:02:06");
        let object = serde_json::to_string(&PhpDateTime::of(time)).expect("the value is JSON");
        assert_eq!(
            object,
            r#"{"date":"2026-11-18 19:02:06.000000","timezone_type":1,"timezone":"+00:00"}"#
        );
    }

    #[test]
    fn uid_is_the_start_of_the_md5_hash_of_the_address() {
        // From Python: hashlib.md5(b"192.0.2.7").hexdigest()[:5], and the same for the
        // other address.
        let v4: IpAddr = "192.0.2.7".parse().expect("the address is valid");
        let v6: IpAddr = "2001:db8::1".parse().expect("the address is valid");

        assert_eq!(uid(v4), "e9cc9");
        assert_eq!(uid(v6), "1d64d");
    }

    fn form() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        headers
    }

    #[test]
    fn token_field_is_read_from_a_form_or_else_from_the_query() {
        let none = HeaderMap::new();

        assert_eq!(
            token_field(Some("Token=abc"), &none, b"").as_deref(),
            Some("abc")
        );
        assert_eq!(
            token_field(Some("a=1&Token=a%20b&z=2"), &none, b"").as_deref(),
            Some("a b")
        );
        assert_eq!(
            token_field(None, &form(), b"Token=from-form").as_deref(),
            Some("from-form")
        );
        // The form wins over the query, as POST wins over GET in PHP.
        assert_eq!(
            token_field(Some("Token=from-query"), &form(), b"Token=from-form").as_deref(),
            Some("from-form")
        );
        assert_eq!(
            token_field(Some("Token=from-query"), &form(), b"other=1").as_deref(),
            Some("from-query")
        );
        assert_eq!(token_field(Some("Token="), &none, b"").as_deref(), Some(""));
    }

    #[test]
    fn token_field_has_exactly_this_name_and_is_not_read_from_json() {
        let mut json = HeaderMap::new();
        json.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        assert_eq!(token_field(Some("token=abc"), &HeaderMap::new(), b""), None);
        assert_eq!(token_field(None, &HeaderMap::new(), b"Token=abc"), None);
        assert_eq!(token_field(None, &json, br#"{"Token": "abc"}"#), None);
        assert_eq!(token_field(None, &HeaderMap::new(), b""), None);
    }
}
