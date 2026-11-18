//! `GET /logout`: ending the session of the browser.

use std::time::SystemTime;

use axum::Extension;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use organizr_auth::SessionToken;

use crate::caller::session_cookie_name;
use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::device_cookie::{DEVICE_COOKIE, new_device_cookie};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::same_origin::FromThisSite;
use crate::state::AppState;

/// Cookies of other applications that a login through Organizr can set. A logout drops
/// them: the tokens of Plex, Ombi, Overseerr, Petio and Jellyfin.
const SINGLE_SIGN_ON_COOKIES: [&str; 5] = ["mpt", "Auth", "oAuth", "connect.sid", "petio_jwt"];

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
    let session_cookie = session_cookie_name(state.settings());
    let token = cookies::cookie_value(&headers, &session_cookie).and_then(SessionToken::parse);
    if let Some(token) = token {
        let existed = state
            .database()
            .sessions()
            .delete_by_token(&token.hash())
            .await?;
        if existed {
            tracing::info!(client = %client.address, "a user logged out");
        }
    }

    // The order is the one of the PHP application.
    let mut dropped = vec![session_cookie.as_str()];
    dropped.extend(SINGLE_SIGN_ON_COOKIES);
    let sent = || cookies::request_cookies(&headers).map(|(name, _)| name);
    // Tautulli names its cookies after its own install, Jellyfin after the user.
    dropped.extend(sent().filter(|name| name.contains("tautulli")));
    dropped.extend(sent().filter(|name| name.contains("user-")));
    dropped.push("jellyfin_credentials");
    dropped.push(DEVICE_COOKIE);

    let now = SystemTime::now();
    let scope = CookieScope::of(&headers, client.https);
    let mut response = Envelope::success(NoData).into_response();
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
