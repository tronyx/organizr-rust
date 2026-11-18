//! `POST /login`: logging in with a name and a password.

use std::time::{Duration, SystemTime};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::login::{self, Device};

use crate::api_data::ApiData;
use crate::caller::{now_stamp, session_cookie_name};
use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::state::AppState;

/// How long a login lasts when the user did not ask to be remembered.
const ONE_DAY: Duration = Duration::from_hours(24);

/// The longest a login lasts, in days, whatever the setting says.
const MOST_DAYS: u32 = 365;

/// `POST /login`: checks a name, or an e-mail address, and a password. When they fit,
/// the answer sets the session cookie.
///
/// Fields of the body: `username`, `password` and, to stay logged in for the number of
/// days of the `rememberMeDays` setting instead of one day, `remember` with any value.
pub async fn login(
    State(state): State<AppState>,
    Extension(client): Extension<Client>,
    headers: HeaderMap,
    data: ApiData,
) -> Result<Response, ApiError> {
    let settings = state.settings();
    // A missing field counts as an empty text, as in the PHP application.
    let name = data.text("username").unwrap_or_default();
    let password = data.text("password").unwrap_or_default();
    let lifetime = if data.is_set("remember") {
        ONE_DAY * settings.remember_me_days.min(MOST_DAYS)
    } else {
        ONE_DAY
    };

    let now = SystemTime::now();
    let device = Device {
        ip: client.address,
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned),
    };

    let logged_in = login::with_password(
        state.database(),
        &name,
        &password,
        device,
        now_stamp(now),
        lifetime,
    )
    .await
    .inspect_err(|error| {
        if matches!(error, login::LoginError::WrongCredentials) {
            tracing::warn!(name = %name, client = %client.address, "a login failed");
        }
    })?;
    tracing::info!(user = %logged_in.user.username, client = %client.address, "a user logged in");

    let session_cookie = CookieScope::of(&headers, client.https).set_hidden(
        &session_cookie_name(settings),
        logged_in.token.expose(),
        now + lifetime,
    )?;
    let mut response = Envelope::success(NoData).into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}
