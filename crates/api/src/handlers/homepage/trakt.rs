//! Trakt on the homepage: the episodes and the films of the calendar of one account,
//! and the login at Trakt that gives Organizr the tokens of that account.

use std::time::SystemTime;

use axum::Extension;
use axum::extract::{RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use organizr_domain::GroupLevel;
use organizr_integrations::http::{Outbound, time_limit};
use organizr_integrations::trakt::{self, Account, Application, Calendar, Grant, Tokens};
use organizr_settings::Settings;
use serde_json::{Value, json};

use super::{Permissions, no_requests, service_url};
use crate::caller::Caller;
use crate::client::Client;
use crate::cookies::cookie_value;
use crate::envelope::Envelope;
use crate::handlers::fields::Refusal;
use crate::handlers::notice_page;
use crate::router::API_BASE;
use crate::state::AppState;

/// The most days that Trakt gives in one calendar.
const MOST_DAYS: u32 = 33;

/// The cookie that holds the `state` of a login at Trakt until the browser comes back.
const STATE_COOKIE: &str = "organizr_trakt_state";

/// How long a login at Trakt may take, in seconds.
const LOGIN_TIME: u32 = 600;

/// What must hold before a caller gets the calendar of Trakt.
pub fn permissions() -> Permissions {
    Permissions {
        enabled: vec!["homepageTraktEnabled".to_owned()],
        auth: vec!["homepageTraktAuth".to_owned()],
        not_empty: vec!["traktClientId".to_owned(), "traktAccessToken".to_owned()],
    }
}

fn now() -> Timestamp {
    Timestamp::try_from(SystemTime::now()).unwrap_or(Timestamp::UNIX_EPOCH)
}

/// The address of this application that the API application at Trakt names.
fn redirect_address(headers: &HeaderMap, client: &Client) -> String {
    service_url(&format!("{API_BASE}/oauth/trakt"), headers, client)
}

/// Stores the tokens of the account. They are secrets of the settings.
async fn store(state: &AppState, tokens: &Tokens) -> Result<(), Refusal> {
    let changes = vec![
        ("traktAccessToken".to_owned(), json!(tokens.access)),
        ("traktAccessTokenExpires".to_owned(), json!(tokens.expires)),
        ("traktRefreshToken".to_owned(), json!(tokens.refresh)),
    ];
    state.change_settings(changes).await.map_err(|error| {
        tracing::error!(%error, "the tokens of Trakt could not be stored");
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "The tokens of Trakt could not be stored".to_owned(),
        )
    })
}

/// The access token for a request: the stored one, or a new one when the stored one
/// is over and Trakt gives new tokens for the refresh token.
async fn access_token(
    state: &AppState,
    settings: &Settings,
    http: &Outbound,
    redirect: &str,
) -> String {
    let stored = settings.trakt_access_token.clone();
    let can_refresh =
        !settings.trakt_refresh_token.is_empty() && !settings.trakt_client_secret.is_empty();
    if !can_refresh || !trakt::is_over(&settings.trakt_access_token_expires, now()) {
        return stored;
    }
    let application = Application {
        api: state.trakt_api(),
        client_id: &settings.trakt_client_id,
        client_secret: &settings.trakt_client_secret,
        redirect,
    };
    let grant = Grant::Refresh(&settings.trakt_refresh_token);
    match trakt::tokens(http, application, grant, now()).await {
        Ok(tokens) => {
            if store(state, &tokens).await.is_err() {
                return stored;
            }
            tokens.access
        }
        Err(error) => {
            tracing::warn!(%error, "the tokens of Trakt could not be renewed");
            stored
        }
    }
}

/// The episodes and the films of the calendar of the account, for the days of the
/// settings.
pub async fn events(
    state: &AppState,
    headers: &HeaderMap,
    client: &Client,
) -> Result<Vec<Value>, Refusal> {
    let settings = state.settings();
    let http = state.outbound().ok_or_else(no_requests)?;
    let redirect = redirect_address(headers, client);
    let token = access_token(state, &settings, http, &redirect).await;
    let account = Account {
        api: state.trakt_api(),
        client_id: &settings.trakt_client_id,
        access_token: &token,
        timeout: time_limit(&settings.calendar_refresh.to_string()),
    };
    let today = now().to_zoned_datetime(TimeZone::system()).date();
    let start = today
        .checked_sub(jiff::Span::new().days(i64::from(settings.calendar_start_trakt)))
        .unwrap_or(today)
        .strftime("%Y-%m-%d")
        .to_string();
    let days = settings
        .calendar_start_trakt
        .saturating_add(settings.calendar_end_trakt)
        .clamp(1, MOST_DAYS);

    let failed = |error: trakt::TraktError| {
        tracing::warn!(%error, "the calendar of Trakt could not be read");
        Refusal::Request(
            StatusCode::INTERNAL_SERVER_ERROR,
            "An error Occurred".to_owned(),
        )
    };
    let mut all = Vec::new();
    let shows = account
        .calendar(http, Calendar::Shows, &start, days)
        .await
        .map_err(failed)?;
    if let Some(shows) = shows {
        all.extend(trakt::show_events(&shows, now()));
    }
    let films = account
        .calendar(http, Calendar::Movies, &start, days)
        .await
        .map_err(failed)?;
    if let Some(films) = films {
        all.extend(trakt::film_events(&films));
    }
    Ok(all)
}

/// `GET /homepage/trakt/calendar`: the episodes and the films of the account.
pub async fn calendar(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
) -> Result<Envelope<Vec<Value>>, Refusal> {
    permissions().check(&state.settings(), &caller)?;
    Ok(Envelope::success(events(&state, &headers, &client).await?))
}

/// A field of a query, with `%xx` and `+` read as their characters.
fn query_field(query: Option<&str>, name: &str) -> Option<String> {
    form_urlencoded::parse(query?.as_bytes())
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

/// The `Set-Cookie` value of the cookie that holds the `state` of a login. An empty
/// value takes the cookie away.
fn state_cookie(value: &str, secure: bool) -> Option<HeaderValue> {
    let age = if value.is_empty() { 0 } else { LOGIN_TIME };
    let secure = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{STATE_COOKIE}={value}; Path={API_BASE}/oauth/trakt; Max-Age={age}; HttpOnly; SameSite=Lax{secure}"
    ))
    .ok()
}

/// `GET /oauth/trakt`: the login of the Admin at Trakt.
///
/// Without a `code`, the browser goes to the page of Trakt where the Admin allows
/// the access. Trakt sends the browser back with a `code`, and the server buys the
/// tokens of the account with it and stores them.
pub async fn oauth(
    State(state): State<AppState>,
    Extension(caller): Extension<Caller>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Result<Response, Refusal> {
    caller.require(GroupLevel::CO_ADMIN)?;
    let settings = state.settings();
    for (key, value) in [
        ("traktClientId", &settings.trakt_client_id),
        ("traktClientSecret", &settings.trakt_client_secret),
    ] {
        if value.is_empty() {
            return Err(Refusal::unprocessable(&format!("{key} was not supplied")));
        }
    }
    let redirect = redirect_address(&headers, &client);
    let query = query.as_deref();

    let Some(code) = query_field(query, "code").filter(|code| !code.is_empty()) else {
        // The state is a random text that only this browser knows. Trakt sends it
        // back, so an answer that another site started is told apart.
        let login = uuid::Uuid::new_v4().simple().to_string();
        let address = trakt::authorize_address(
            state.trakt_site(),
            &settings.trakt_client_id,
            &redirect,
            &login,
        );
        let mut response = (StatusCode::FOUND, [(header::LOCATION, address)]).into_response();
        if let Some(cookie) = state_cookie(&login, client.https) {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        return Ok(response);
    };

    let sent = query_field(query, "state").unwrap_or_default();
    let expected = cookie_value(&headers, STATE_COOKIE).unwrap_or_default();
    if sent.is_empty() || sent != expected {
        return Ok(notice_page::plain(StatusCode::OK, "Invalid state"));
    }
    let http = state.outbound().ok_or_else(no_requests)?;
    let application = Application {
        api: state.trakt_api(),
        client_id: &settings.trakt_client_id,
        client_secret: &settings.trakt_client_secret,
        redirect: &redirect,
    };
    let mut response = match trakt::tokens(http, application, Grant::Code(&code), now()).await {
        Ok(tokens) => {
            store(&state, &tokens).await?;
            notice_page::page(StatusCode::OK, "Trakt OAuth", "Details Saved", true)
        }
        Err(error) => {
            tracing::warn!(%error, "the login at Trakt failed");
            notice_page::plain(StatusCode::OK, &error.to_string())
        }
    };
    if let Some(cookie) = state_cookie("", client.https) {
        response.headers_mut().append(header::SET_COOKIE, cookie);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::{query_field, state_cookie};

    #[test]
    fn field_of_a_query_is_read_with_its_characters() {
        let query = Some("code=a%2Fb+c&state=xyz&empty=");
        assert_eq!(query_field(query, "code").as_deref(), Some("a/b c"));
        assert_eq!(query_field(query, "state").as_deref(), Some("xyz"));
        assert_eq!(query_field(query, "empty").as_deref(), Some(""));
        assert_eq!(query_field(query, "other"), None);
        assert_eq!(query_field(None, "code"), None);
    }

    #[test]
    fn cookie_of_a_login_is_for_the_route_of_the_login_only() {
        let cookie = state_cookie("abc", true).expect("a cookie");
        assert_eq!(
            cookie,
            "organizr_trakt_state=abc; Path=/api/v2/oauth/trakt; Max-Age=600; HttpOnly; SameSite=Lax; Secure"
        );
        let gone = state_cookie("", false).expect("a cookie");
        assert_eq!(
            gone,
            "organizr_trakt_state=; Path=/api/v2/oauth/trakt; Max-Age=0; HttpOnly; SameSite=Lax"
        );
    }
}
