//! `GET /logout`: ending the session of the browser.

use std::time::SystemTime;

use axum::Extension;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use organizr_auth::SessionToken;

use crate::caller::{MOST_SESSION_COOKIES, session_cookie_name};
use crate::client::Client;
use crate::cookies;
use crate::device_cookie::{DEVICE_COOKIE, new_device_cookie};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::same_origin::FromThisSite;
use crate::state::AppState;

/// Cookies of other applications that a login through Organizr can set. A logout drops
/// them: the tokens of Plex, Ombi, Overseerr, Petio and Jellyfin.
const SINGLE_SIGN_ON_COOKIES: [&str; 5] = ["mpt", "Auth", "oAuth", "connect.sid", "petio_jwt"];

/// The longest wait for Komga at a logout.
const KOMGA_LOGOUT_TIME: std::time::Duration = std::time::Duration::from_secs(10);

/// Asks Komga to end the session of `token`, as PHP's `clearKomgaToken` does.
async fn end_komga_session(state: &AppState, token: &str) {
    let settings = state.settings();
    let Some(http) = state.outbound() else {
        return;
    };
    if settings.komga_url.is_empty() {
        return;
    }
    let url = crate::handlers::homepage::service_url(&settings.komga_url);
    organizr_integrations::sso::komga_logout(http, &url, token, KOMGA_LOGOUT_TIME).await;
}

/// `GET /logout`: deletes the session of the request and makes the browser drop the
/// session cookie and the cookies of other applications. The browser gets a new device
/// id.
///
/// The answer is the same whether or not the caller was logged in. Because the route
/// changes something with a GET request, it takes requests from the application itself
/// only; see [`FromThisSite`].
pub async fn logout(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    _: FromThisSite,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let session_cookie = session_cookie_name(&state.settings());
    // Every session cookie of the name counts, up to a few: another host of the cookie
    // domain could have planted one that comes first (D-145).
    let tokens = cookies::cookie_values(&headers, &session_cookie)
        .take(MOST_SESSION_COOKIES)
        .filter_map(SessionToken::parse);
    for token in tokens {
        let existed = state
            .database()
            .sessions()
            .delete_by_token(&token.hash())
            .await?;
        if existed {
            tracing::info!(client = %client.address, "a user logged out");
        }
    }

    // The order is the one of the PHP application: the session cookie first.
    let mut dropped: Vec<&str> = SINGLE_SIGN_ON_COOKIES.to_vec();
    let sent = || cookies::request_cookies(&headers).map(|(name, _)| name);
    // Tautulli names its cookies after its own install, Jellyfin after the user.
    dropped.extend(sent().filter(|name| name.contains("tautulli")));
    dropped.extend(sent().filter(|name| name.contains("user-")));
    dropped.push("jellyfin_credentials");
    // A session of Komga ends at Komga too, and its cookie is dropped only when the
    // browser sent it, as in PHP.
    if let Some(token) = cookies::cookie_value(&headers, "komga_token") {
        end_komga_session(&state, token).await;
        dropped.push("komga_token");
    }
    dropped.push(DEVICE_COOKIE);

    let now = SystemTime::now();
    let scope = state.cookie_scope(&headers, client.https);
    let mut response = Envelope::success(NoData).into_response();
    cookies::append(&mut response, scope.delete_session(&session_cookie, now)?);
    for name in dropped {
        match scope.delete(name, now) {
            Ok(values) => cookies::append(&mut response, values),
            // A name that a header cannot carry. The browser keeps that cookie.
            Err(error) => tracing::debug!(%error, "a cookie could not be dropped at logout"),
        }
    }
    // After the deletion above, so the browser ends up with the new id.
    cookies::append(&mut response, new_device_cookie(&scope, now)?);
    Ok(response)
}
