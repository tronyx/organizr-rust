//! `POST /login`: logging in with a name and a password.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Extension;
use axum::extract::State;
use axum::http::{HeaderMap, header};
use axum::response::{IntoResponse, Response};
use organizr_auth::login::{self, Device};
use organizr_domain::Timestamp;
use organizr_settings::Settings;

use crate::api_data::ApiData;
use crate::client::Client;
use crate::cookies::{self, CookieScope};
use crate::envelope::{Envelope, NoData};
use crate::error::ApiError;
use crate::state::AppState;

/// How long a login lasts when the user did not ask to be remembered.
const ONE_DAY: Duration = Duration::from_hours(24);

/// The longest a login lasts, in days, whatever the setting says.
const MOST_DAYS: u32 = 365;

/// The name of the session cookie. It ends with the id of the install, so two installs
/// on one domain do not take each other's cookie.
pub fn session_cookie_name(settings: &Settings) -> String {
    if settings.install_id.is_empty() {
        "organizr_token_temp".to_owned()
    } else {
        format!("organizr_token_{}", settings.install_id)
    }
}

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
    let since_epoch = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let now_stamp =
        Timestamp::from_unix_seconds(i64::try_from(since_epoch.as_secs()).unwrap_or(i64::MAX));
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
        now_stamp,
        lifetime,
    )
    .await
    .inspect_err(|error| {
        if matches!(error, login::LoginError::WrongCredentials) {
            tracing::warn!(name = %name, client = %client.address, "a login failed");
        }
    })?;
    tracing::info!(user = %logged_in.user.username, client = %client.address, "a user logged in");

    let session_cookie = CookieScope::of(&headers, client.https).set(
        &session_cookie_name(settings),
        logged_in.token.expose(),
        now + lifetime,
    )?;
    let mut response = Envelope::success(NoData).into_response();
    cookies::append(&mut response, session_cookie);
    Ok(response)
}

#[cfg(test)]
mod tests {
    use organizr_settings::Settings;
    use serde_json::json;

    use super::session_cookie_name;

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
}
